use arbutil::crypto;
use prover::{binary::parse, programs::{start::StartlessMachine, config::{StylusConfig, StylusDebugConfig}, counter::CountingMachine, STYLUS_ENTRY_POINT}};
use core::panic;
use std::time::{Duration, Instant};
use stylus::{env::WasmEnv, stylus::{instance_from_module, NativeInstance, instance}};
use wasmer::Module;
use wasmparser::{Validator, WasmFeatures, Operator};

macro_rules! wat {
    ($wasm:expr) => {{
        let wat = wabt::Wasm2Wat::new()
            .fold_exprs(true)
            .inline_export(true)
            .convert($wasm);
        match wat {
            Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
            Err(err) => format!("wasm2wat failed: {}", err),
        }
    }};
}

pub fn validate(input: &[u8]) -> bool {
    let features = WasmFeatures {
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
    };
    let mut validator = Validator::new();
    validator.wasm_features(features);

    validator.validate_all(input).is_ok()
}
    

fn uniform_cost_config() -> StylusConfig {
    let mut config = StylusConfig::default();
    config.add_debug_params();
    config.start_gas = 1_000_000;
    config.pricing.wasm_gas_price = 100_00;
    config.pricing.hostio_cost = 100;
    config.costs = |_| 1;
    config
}

fn main() {
    //let filename = "tests/keccak/target/wasm32-unknown-unknown/release/keccak.wasm";
    //let filename = "tests/siphash/siphash.wasm";

    // let file = include_bytes!("../../stylus/tests/keccak/target/wasm32-unknown-unknown/release/keccak.wasm");
    // let wasm_data = file.to_vec(); //todo
    // assert!(validate(&wasm_data));

    let enable_counter = false;
    let gas_limit = 200;


    let mut config = StylusConfig::default();
    if enable_counter {
        config.add_debug_params();
    }
    config.costs = |_: &Operator| -> u64 {1};
    config.start_gas = gas_limit;
    config.pricing.wasm_gas_price = 1;

    // let module = match Module::new(&config.store(), wasm_data.clone()) {
    //     Ok(module) => module,
    //     Err(_) => {
    //         todo!()
    //         // return Corpus::Keep;
    //     }
    // };

    // let preimage = "°º¤ø,¸,ø¤°º¤ø,¸,ø¤°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan";
    // let preimage = preimage.as_bytes().to_vec();
    // let hash = hex::encode(crypto::keccak(&preimage));

    // let mut args = vec![0x01];
    // args.extend(preimage);
    // let args_len = args.len() as u32;


    // let env = WasmEnv::new(config.clone(), args);
    // let mut instance = match instance_from_module(module, config.store(), env) {
    //     Ok(instance) => instance,
    //     Err(err) => {
    //         let err = err.to_string();
    //         if err.contains("Missing export memory") ||
    //            err.contains("out of bounds memory access") ||
    //            err.contains("Incompatible Export Type") ||
    //            err.contains("WebAssembly transaction error") ||
    //            err.contains("out of bounds table access") {
    //             todo!()
    //             // return Corpus::Keep;
    //         }
    //         println!("{}", wat!(&wasm_data));
    //         panic!("Failed to create instance: {err}");
    //     }
    // };

    // // let starter = match instance.get_start() {
    // //     Ok(starter) => starter,
    // //     Err(err) => {
    // //         // println!("{}", wat!(&wasm_data));
    // //         panic!("Failed to get start: {err}");
    // //     }
    // // };
    // let store = &instance.store;
    // let exports = &instance.instance.exports;
    // let main = match exports.get_typed_function::<u32, i32>(&store, STYLUS_ENTRY_POINT) {
    //     Ok(main) => main, 
    //     Err(err) => {
    //         panic!("Failed to get start: {err}");
    //     }
    // };
    // // let status = main.call(store, args_len)?;

    // if let Err(e) = main.call(&mut instance.store, args_len) {
    //     println!("{}", wat!(&wasm_data));
    //     panic!("Failed to run: {e}");
    // }
    // println!("Finished main");

    // if enable_counter {
    //     let counts = match instance.operator_counts() {
    //         Ok(counts) => counts,
    //         Err(err) => {
    //             println!("{}", wat!(&wasm_data));
    //             panic!("Failed to get operator counts: {err}");
    //         }
    //     };
    //     for (op, count) in counts.into_iter() {
    //         println!("{op}\t{count}\n");
    //     }
    // }

    let filename = "../stylus/tests/keccak/target/wasm32-unknown-unknown/release/keccak.wasm";
    // let pathcontents : Vec<_> = std::fs::read_dir("..").unwrap().collect();
    // dbg!(pathcontents);
    let preimage = "°º¤ø,¸,ø¤°º¤ø,¸,ø¤°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan";
    let preimage = preimage.as_bytes().to_vec();
    let hash = hex::encode(crypto::keccak(&preimage));

    let mut args = vec![0x01];
    args.extend(preimage);
    let args_len = args.len() as u32;
    let config = uniform_cost_config();

    let env = WasmEnv::new(config.clone(), args.clone());
    let mut native = match stylus::stylus::instance(filename, env) {
        Ok(native) => native, 
        Err(err) => panic!("failed to create instance. err was {err}"),
    };
    let exports = &native.instance.exports;
    let store = &mut native.store;

    let main = match exports.get_typed_function::<u32, i32>(store, STYLUS_ENTRY_POINT) {
        Ok(main) => main,
        Err(err) => panic!("failed to get main. err was {err}"),
    };

    //start timing
    let start = Instant::now();
    let status = match main.call(store, args_len) {
        Ok(status) => status, 
        Err(err) => panic!("failed to run main. err was {err}"),
    };
    let duration = start.elapsed();
    //end timing
    assert_eq!(status, 0);

    println!("Time elapsed in running program() is: {:?}", duration);
    
    let env = native.env.as_ref(&store);
    assert_eq!(hex::encode(&env.outs), hash);

    let counts = match native.operator_counts() {
        Ok(counts) => counts,
        Err(err) => {
            panic!("Failed to get operator counts: {err}");
        }
    };
    for (op, count) in counts.into_iter() {
        println!("{op}\t{count}\n");
    }

}
