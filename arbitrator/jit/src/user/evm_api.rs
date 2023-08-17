// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![allow(clippy::too_many_arguments)]

use crate::{
    gostack::GoStack,
    machine::WasmEnvMut,
    syscall::{DynamicObject, GoValue, JsValue, STYLUS_ID},
    user::{DownMsg, StylusThreadHandler, UpMsg, StylusLaunchParams},
};
use arbutil::{
    evm::{
        js::{ApiValue, JsCallIntoGo, JsEvmApi},
        user::UserOutcome,
        EvmData,
    },
    Color,
};
use eyre::{bail, Result};
use prover::programs::prelude::*;
use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use stylus::{native::NativeInstance, run::RunProgram};

struct StylusThreadData {
    up: SyncSender<UpMsg>,
    down: Mutex<Receiver<DownMsg>>,
}

#[derive(Clone)]
struct StylusThread {
    data: Arc<StylusThreadData>,
}

impl StylusThread {
    fn new(
        up: SyncSender<UpMsg>,
        down: Receiver<DownMsg>,
    ) -> Self {
        Self {
            data: Arc::new(StylusThreadData {
                up,
                down: down.into(),
            }),
        }
    }

    unsafe fn launch_new_wavm(
        &self,
        params: StylusLaunchParams,
    ) -> (Result<UserOutcome>, u64) {
        let evm_api = JsEvmApi::new(params.evm_api_ids.clone(), self.clone());
        let instance =
            NativeInstance::deserialize(&params.module, params.compile.clone(), evm_api, params.evm_data);
        let mut instance = match instance {
            Ok(instance) => instance,
            Err(error) => {
                let message = format!("failed to instantiate program {error:?}");
                self.data.up.send(UpMsg::Panic(message.clone())).unwrap();
                panic!("{message}");
            }
        };

        let outcome = instance.run_main(&params.calldata, params.config, params.ink);

        let ink_left: u64 = match outcome {
            Ok(UserOutcome::OutOfStack) => 0, // take all ink when out of stack
            _ => instance.ink_left().into(),
        };
        (outcome, ink_left)
    }
}

impl JsCallIntoGo for StylusThread {
    fn call_go(&mut self, func: u32, args: Vec<ApiValue>) -> Vec<ApiValue> {
        unsafe {
            self.data
                .up
                .send(UpMsg::Call(func, args))
                .expect("failed sending from stylus thread to go");
            loop {
                let msg = self.data.down.lock().unwrap().recv().unwrap();
                match msg {
                    DownMsg::CallResponse(res) => return res,
                    DownMsg::ExecWasm(params) => {
                        let (outcome, ink_left) =
                            self.launch_new_wavm(params);
                        self.data
                            .up
                            .send(UpMsg::WasmDone(outcome, ink_left))
                            .expect("failed sending from stylus thread to go");
                    }
                }
            }
        }
    }
}

impl StylusThreadHandler {
    fn increase_call(
        &mut self,
        timeout: Duration,
    ) {
        self.calls += 1;
        if self.calls > 1 {
            return;
        }
        let up_channel = mpsc::sync_channel(0);
        let down_channel = mpsc::sync_channel(0);
        let handler: thread::JoinHandle<()> = thread::spawn(move || unsafe {
            let thread =
                StylusThread::new(up_channel.0, down_channel.1);
            // Safety: module came from compile_user_wasm
            let msg = thread.data.down.lock().unwrap().recv().unwrap();
            let DownMsg::ExecWasm(params) = msg else {
                panic!("stylus thread got wrong message")
            };
            let (outcome, ink_left) = thread.launch_new_wavm(params);
            thread
                .data
                .up
                .send(UpMsg::WasmDone(outcome, ink_left))
                .expect("failed replying from stylus thread");
        });
        self.thread_info = Some((down_channel.0, up_channel.1, handler));
        self.timeout = timeout;
    }

    fn decrease_call(&mut self) {
        self.calls -= 1;
        if self.calls > 0 {
            return;
        }
        let (_, _, handler) = self.thread_info.take().expect("stylus thread not found");
        handler.join().expect("failed joining stylus thread");
    }

    fn send(&mut self, msg: DownMsg) -> Result<()> {
        let (ref down, _, _) = self.thread_info.as_mut().expect("stylus thread not found");
        match down.send(msg) {
            Ok(_) => Ok(()),
            Err(err) => bail!("{}", err.red()),
        }
    }

    fn recv(&mut self) -> Result<UpMsg> {
        let (_, ref up, _) = self.thread_info.as_mut().expect("stylus thread not found");
        match up.recv_timeout(self.timeout) {
            Ok(msg) => Ok(msg),
            Err(err) => bail!("{}", err.red()),
        }
    }
}

/// Executes a wasm on a new thread
pub(super) fn exec_wasm(
    sp: &mut GoStack,
    mut env: WasmEnvMut,
    module: Vec<u8>,
    calldata: Vec<u8>,
    compile: CompileConfig,
    config: StylusConfig,
    evm_api_ids: Vec<u8>,
    evm_data: EvmData,
    ink: u64,
) -> Result<(Result<UserOutcome>, u64)> {
    use UpMsg::*;

    let (env, mut store) = env.data_and_store_mut();

    env.stylus_thread_handler
        .increase_call(env.process.child_timeout);

    env.stylus_thread_handler
        .send(DownMsg::ExecWasm(StylusLaunchParams{evm_api_ids, evm_data, module, calldata, ink, compile, config}))?;

    loop {
        let msg = env.stylus_thread_handler.recv()?;
        match msg {
            Call(func, args) => {
                let js = &mut env.js_state;

                let mut objects = vec![];
                let mut object_ids = vec![];
                for arg in args {
                    let id = js.pool.insert(DynamicObject::Uint8Array(arg.0));
                    objects.push(GoValue::Object(id));
                    object_ids.push(id);
                }

                let Some(DynamicObject::FunctionWrapper(func)) = js.pool.get(func).cloned() else {
                    bail!("missing func {}", func.red())
                };

                js.set_pending_event(func, JsValue::Ref(STYLUS_ID), objects);
                unsafe { sp.resume(env, &mut store)? };

                let js = &mut env.js_state;
                let Some(JsValue::Ref(output)) = js.stylus_result.take() else {
                    bail!("no return value for func {}", func.red())
                };
                let Some(DynamicObject::ValueArray(output)) = js.pool.remove(output) else {
                    bail!("bad return value for func {}", func.red())
                };

                let mut outs = vec![];
                for out in output {
                    let id = out.assume_id()?;
                    let Some(DynamicObject::Uint8Array(x)) = js.pool.remove(id) else {
                        bail!("bad inner return value for func {}", func.red())
                    };
                    outs.push(ApiValue(x));
                }

                for id in object_ids {
                    env.js_state.pool.remove(id);
                }
                env.stylus_thread_handler
                    .send(DownMsg::CallResponse(outs))?;
            }
            Panic(error) => bail!(error),
            WasmDone(res, ink_left) => {
                env.stylus_thread_handler.decrease_call();
                return Ok((res, ink_left));
            }
        }
    }
}
