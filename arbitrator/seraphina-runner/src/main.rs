use arbutil::crypto;
use core::panic;
use prover::{
    binary::parse,
    programs::{
        config::{StylusConfig},
        counter::CountingMachine,
        start::StartlessMachine,
        STYLUS_ENTRY_POINT,
    },
};
use regex;
use stylus::{
    env::WasmEnv,
    native::NativeInstance,
};
use wasmer::Module;
use wasmparser::{Operator, Validator, WasmFeatures};

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

    let res = validator.validate_all(input);
    println!("val res: {:?}", res);
    // if let Err(e) = res.clone() {
    //     println!("{}", e);
    // } else {
    //     println!("validated");
    // }
    res.is_ok()
}

fn add_debug_params(config: &mut StylusConfig) {
    config.debug.debug_funcs = true; 
    config.debug.count_ops = true;
}

fn uniform_cost_config() -> StylusConfig {
    let mut config = StylusConfig::default();
    add_debug_params(&mut config);
    config.start_gas = 1_000_000;
    config.pricing.wasm_gas_price = 100_00;
    config.pricing.hostio_cost = 100;
    config.costs = |_| 1;
    config
}

fn rewrite_memory_line(line: String) -> String {
    let prefix = "(memory (;0;) (export \"";
    if !line.starts_with(prefix) {
        println!("nonconforming line: .{}.", line);
        panic!();
    } else {
        let re = regex::Regex::new("\"[^\"]+\"").expect("static");
        return re.replace(&line, "\"memory\"").to_string();
    }
}

fn modify_wat(wat: String) -> String {
    let mut out = vec![];
    let mut count = 0;
    for line in wat.lines() {
        let line = line.trim().to_string();
        println!("line {} was .{}.", count, line);
        if line.starts_with("(memory") {
            let line = rewrite_memory_line(line);
            println!("line was modified to .{}", line);
            out.push(line);
        } else {
            out.push(line);
        }
        count += 1;
    }
    let ans = out.join("\n");
    println!("ans: {}", ans);
    ans
}

fn fuzz_me(wasm_data: &[u8]) {
    let wat = match wabt::Wasm2Wat::new()
        .fold_exprs(true)
        .inline_export(true)
        .convert(&wasm_data)
    {
        Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
        Err(err) => format!("wasm2wat failed: {}", err),
    };
    let new_wat = modify_wat(wat);
    let wasm_data = wabt::wat2wasm(new_wat).unwrap();
    // let round_trip_wasm = wabt::wat2wasm(&wat).unwrap();
    // if wasm_data != round_trip_wasm {
    //     let wat2 = match wabt::Wasm2Wat::new()
    //     .fold_exprs(true)
    //     .inline_export(true)
    //     .convert(&wasm_data)
    //     {
    //         Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
    //         Err(err) => format!("wasm2wat failed: {}", err),
    //     };
    //     println!("fst: {} \nsnd: {}", wat, wat2);
    //     panic!("wats differed");
    // }

    if !validate(&wasm_data) {
        panic!("wasm didn't validate!");
    }
    let enable_counter = false;
    let gas_limit = 200;

    let mut config = StylusConfig::default();
    if enable_counter {
        add_debug_params(&mut config);
    }
    config.costs = |_: &Operator| -> u64 { 1 };
    config.start_gas = gas_limit;
    config.pricing.wasm_gas_price = 1;

    let module = match Module::new(&config.store(), wasm_data.clone()) {
        Ok(module) => module,
        Err(err) => {
            println!("error in module creation: {}", err);
            panic!();
        }
    };
    // println!("hi1");
    let env = WasmEnv::new(config.clone());
    let mut instance = match NativeInstance::from_module(module, config.store(), env) {
        Ok(instance) => instance,
        Err(err) => {
            let err = err.to_string();

            if err.contains("Missing export memory")
                || err.contains("out of bounds memory access")
                || err.contains("out of bounds table access")
            //    err.contains("Incompatible Export Type") ||
            //    err.contains("WebAssembly transaction error") ||
            {
                println!("instance err: {}", err);
                panic!();
            }

            dbg!(&err);
            println!("wat = {}", wat!(&wasm_data));
            panic!("Failed to create instance: {}", err);
        }
    };
    println!("wat = {}", wat!(&wasm_data));
    println!("hi2");
    let starter = match instance.get_start() {
        Ok(starter) => starter,
        Err(err) => {
            println!("{}", wat!(&wasm_data));
            // panic!("Failed to get start: {err}");
            println!("Failed to get start: {err}");
            panic!();
        }
    };
    println!("hi3");
    // println!("starter func: {:?}", starter);
    if let Err(e) = starter.call(&mut instance.store) {
        println!("{}", wat!(&wasm_data));
        println!("Failed to run start: {e}");
        panic!();
    }
    println!("Finished main");

    if enable_counter {
        let counts = match instance.operator_counts() {
            Ok(counts) => counts,
            Err(err) => {
                println!("{}", wat!(&wasm_data));
                panic!("Failed to get operator counts: {err}");
            }
        };
        for (op, count) in counts.into_iter() {
            println!("{op}\t{count}\n");
        }
    }
    println!("run succeeded!");
}

fn main() {
    let mywat = include_str!("my_wat.wat");
    let mywasm = wabt::wat2wasm(mywat).expect("it's a valid wat");
    fuzz_me(&mywasm);

    //     let enable_counter = false;
    //     let gas_limit = 200;

    //     let mut config = StylusConfig::default();
    //     if enable_counter {
    //         config.add_debug_params();
    //     }
    //     config.costs = |_: &Operator| -> u64 {1};
    //     config.start_gas = gas_limit;
    //     config.pricing.wasm_gas_price = 1;

    //     let filename = "../stylus/tests/keccak/target/wasm32-unknown-unknown/release/keccak.wasm";
    //     // let pathcontents : Vec<_> = std::fs::read_dir("..").unwrap().collect();
    //     // dbg!(pathcontents);
    //     let preimage = "°º¤ø,¸,ø¤°º¤ø,¸,ø¤°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan";
    //     let preimage = preimage.as_bytes().to_vec();
    //     let hash = hex::encode(crypto::keccak(&preimage));

    //     let mut args = vec![0x01];
    //     args.extend(preimage);
    //     let args_len = args.len() as u32;
    //     let config = uniform_cost_config();

    //     let env = WasmEnv::new(config.clone(), args.clone());
    //     let mut native = match stylus::stylus::instance(filename, env) {
    //         Ok(native) => native,
    //         Err(err) => panic!("failed to create instance. err was {err}"),
    //     };
    //     let exports = &native.instance.exports;
    //     let store = &mut native.store;

    //     let main = match exports.get_typed_function::<u32, i32>(store, STYLUS_ENTRY_POINT) {
    //         Ok(main) => main,
    //         Err(err) => panic!("failed to get main. err was {err}"),
    //     };

    //     //start timing
    //     let start = Instant::now();
    //     let status = match main.call(store, args_len) {
    //         Ok(status) => status,
    //         Err(err) => panic!("failed to run main. err was {err}"),
    //     };
    //     let duration = start.elapsed();
    //     //end timing
    //     assert_eq!(status, 0);

    //     println!("Time elapsed in running program() is: {:?}", duration);

    //     let env = native.env.as_ref(&store);
    //     assert_eq!(hex::encode(&env.outs), hash);

    //     let counts = match native.operator_counts() {
    //         Ok(counts) => counts,
    //         Err(err) => {
    //             panic!("Failed to get operator counts: {err}");
    //         }
    //     };
    //     for (op, count) in counts.into_iter() {
    //         println!("{op}\t{count}\n");
    //     }
}
