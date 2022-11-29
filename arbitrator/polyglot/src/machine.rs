// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use prover::programs::{
    meter::{MachineMeter, MeteredMachine},
    PolyglotConfig,
};

use eyre::Result;
use parking_lot::Mutex;
use thiserror::Error;
use wasmer::{
    imports, Function, Global, Instance, Memory, MemoryView, Module, RuntimeError, Store, WasmerEnv,
};

use std::{ops::Deref, sync::Arc};

pub fn validate(wasm: &[u8]) -> Result<()> {
    let features = wasmparser::WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: false,
        module_linking: false,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        deterministic_only: false,
        multi_memory: false,
        exceptions: false,
        memory64: false,
        extended_const: false,
        //component_model: false, TODO: add in 0.84
    };
    let mut validator = wasmparser::Validator::new();
    validator.wasm_features(features);
    validator.validate_all(wasm)?;
    Ok(())
}

pub fn instrument(wasm: &[u8], config: &PolyglotConfig) -> Result<(Vec<u8>, Store)> {
    let store = config.store()?;
    let module = Module::new(&store, wasm)?;
    let module = module.serialize()?;
    Ok((module, store))
}

pub fn create(module: &[u8], store: &Store, env: WasmEnvArc) -> Result<Instance> {
    //let store = Store::new(engine);
    let module = unsafe { Module::deserialize(&store, module)? };

    macro_rules! func {
        ($func:expr) => {
            Function::new_native_with_env(&store, env.clone(), $func)
        };
    }
    let imports = imports! {
        "env" => {
            "read_args" => func!(read_args),
            "return_data" => func!(return_data),
        }
    };

    fn expect_global(instance: &Instance, name: &str) -> Global {
        instance.exports.get_global(name).unwrap().clone()
    }

    let instance = Instance::new(&module, &imports)?;
    let gas_left_global = expect_global(&instance, "polyglot_gas_left");
    let gas_status_global = expect_global(&instance, "polyglot_gas_status");

    let mut env = env.lock();
    env.memory = instance.exports.get_memory("memory").cloned().ok();
    env.gas_globals = Some((gas_left_global, gas_status_global));
    Ok(instance)
}

#[derive(Clone, Default, WasmerEnv)]
pub struct WasmEnvArc {
    env: Arc<Mutex<WasmEnv>>,
}

#[derive(Default)]
pub struct WasmEnv {
    pub args: Vec<u8>,
    pub outs: Vec<u8>,
    /// The price of wasm gas, measured in thousandths of an evm gas
    gas_price: u64,
    /// Mechanism for reading and writing the module's memory
    memory: Option<Memory>,
    /// Mechanism for reading and writing the amount of gas left
    gas_globals: Option<(Global, Global)>,
}

impl Deref for WasmEnvArc {
    type Target = Mutex<WasmEnv>;
    fn deref(&self) -> &Self::Target {
        &*self.env
    }
}

impl WasmEnvArc {
    pub fn new(args: &[u8], gas_price: u64) -> Self {
        assert!(gas_price > 0, "gas price must be > 0");
        let mut env = WasmEnv::default();
        env.args = args.to_owned();
        env.gas_price = gas_price;
        WasmEnvArc {
            env: Arc::new(Mutex::new(env)),
        }
    }
}

impl WasmEnv {
    fn read_slice(&self, ptr: usize, len: usize) -> Vec<u8> {
        let memory = self.memory.clone().expect("no memory");
        unsafe { memory.data_unchecked()[ptr..ptr + len].to_vec() }
    }

    fn write_slice(&self, ptr: u32, src: &[u8]) {
        let memory = self.memory.clone().expect("no memory");
        let view: MemoryView<u8> = memory.view();
        let view = view.subarray(ptr, ptr + src.len() as u32);
        unsafe { view.copy_from(src) }
    }

    pub fn buy_evm_gas(&mut self, evm_gas: u64) -> MaybeEscape {
        let MachineMeter::Ready(mut gas_left) = self.gas_left() else {
            return Escape::out_of_gas()
        };

        let mut evm_gas_left = gas_left.saturating_mul(1000) / self.gas_price;
        if evm_gas > evm_gas_left {
            let (_, status) = self.gas_globals.as_ref().unwrap();
            status.set(1.into())?;
            return Escape::out_of_gas();
        }
        evm_gas_left -= evm_gas;
        gas_left = evm_gas_left.saturating_mul(1000) / self.gas_price;
        self.set_gas(gas_left);
        Ok(())
    }
}

impl MeteredMachine for WasmEnv {
    fn gas_left(&self) -> MachineMeter {
        let (gas_left, status) = self.gas_globals.as_ref().unwrap();
        if status.get() == 1.into() {
            return MachineMeter::Exhausted;
        }
        MachineMeter::Ready(gas_left.get().try_into().unwrap())
    }
    fn set_gas(&mut self, gas: u64) {
        let (global, _) = self.gas_globals.as_ref().unwrap();
        global.set(gas.into()).unwrap();
    }
}

#[derive(Error, Debug)]
pub enum Escape {
    #[error("program exited with status code `{0}`")]
    Exit(u32),
    #[error("runtime failed with `{0}`")]
    Failure(String),
    #[error("hostio failed with `{0}`")]
    HostIO(String),
    #[error("out of gas")]
    OutOfGas,
}

impl From<RuntimeError> for Escape {
    fn from(outcome: RuntimeError) -> Self {
        match outcome.downcast() {
            Ok(escape) => escape,
            Err(outcome) => Escape::Failure(format!("unknown runtime error: {outcome}")),
        }
    }
}

pub type MaybeEscape = Result<(), Escape>;

impl Escape {
    pub fn exit(code: u32) -> MaybeEscape {
        Err(Self::Exit(code))
    }

    pub fn hostio<S: std::convert::AsRef<str>>(message: S) -> MaybeEscape {
        Err(Self::HostIO(message.as_ref().to_string()))
    }

    pub fn failure<S: std::convert::AsRef<str>>(message: S) -> MaybeEscape {
        Err(Self::Failure(message.as_ref().to_string()))
    }

    pub fn out_of_gas() -> MaybeEscape {
        Err(Self::OutOfGas)
    }
}

type Pointer = u32;

fn read_args(env: &WasmEnvArc, dest: Pointer) {
    let env = env.lock();
    env.write_slice(dest, &env.args);
}

fn return_data(env: &WasmEnvArc, len: u32, data: Pointer) -> MaybeEscape {
    let env = &mut *env.lock();

    let evm_words = |count: u64| count.saturating_add(31) / 32;
    let evm_gas = evm_words(len.into()).saturating_mul(3); // each byte is 3 evm gas per evm word
    env.buy_evm_gas(evm_gas)?;

    env.outs = env.read_slice(data as usize, len as usize);
    Ok(())
}
