/*
    Goal: price at least i32.add and i32.mul instructions. 
    Strategy: generate straight line code that adds and muls a lot. 
        this will need a way to make the stack larger, which we will 
        do with i32.const and local.get, and to mix values around, we 
        will also generate local.set and local.tee. 

    Details: 
        a "payload" is a desired instruction to include (will eventually be more 
        than 2). We start by randomly generating a list of payloads, plus a skeleton
        to go around it that has things like an entrypoint and declares locals (1-10?). 

        We then turn that list of payloads into a list of validly to execute
        wasm opcodes. A payload typically consumes some elements from the stack, so 
        as we do this, we keep track of the current height of the stack and add elements 
        to the stack if they are needed, via a set of auxilary instructions that add a 
        fixed number of elements to the stack. A basic set of these instructions might 
        be i32.const and local.get. 

        To add slightly more depth to the program, we also have a chance of storing the 
        values on the stack back to the local variables. 

        As we output each opcode into the generated file, we can count it, and thereby 
        obtain an accurate count of the number of opcodes of each type in the file 
        (each opcode is only executed once, since the code is straight-line). We can then
        run each generated file through a timer to obtain aggregate timing data and use 
        linear regression to obtain timings per-opcode. 

    Future work: 
        loops and other control flow
        
        expand the set of payloads

        payloads can be more than one opcode (eg 3 adds in a row), which makes the synthetic
        workload somewhat more realistic
 */

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use rand::prelude::*;
use rand::Rng;
use rand::distributions::WeightedIndex;
use rand_pcg::Pcg32;
use wasmer::Cranelift;
use wasmer::CraneliftOptLevel;
use wasmer::{Store, Module, Instance, Value, imports};
use wasmer::FunctionEnv;
use anyhow;

mod rng;
use crate::rng::Lcg64XshRR32;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Opcode {
    I32Add,
    I32Mul,
    Drop,
    I32Const,
    LocalSet, 
    LocalGet, 
    LocalTee,
}
use Opcode::*;

impl Opcode {
    fn to_str(&self) -> &str {
        match self {
            I32Add => "i32.add",
            I32Mul => "i32.mul",
            Drop => "drop",
            I32Const => "i32.const",
            LocalSet => "local.set",
            LocalGet => "local.get",
            LocalTee => "local.tee",
        }
    }

    // fn requires_arg(&self) -> bool {
    //     match self {
    //         I32Add => false,
    //         I32Mul => false,
    //         Drop => false,
    //         I32Const => true,
    //         LocalSet => true,
    //         LocalGet => true,
    //         LocalTee => true,
    //     }
    // }
}

// first result is how large the stack must be. second result is net change in stack. 
fn opcode_stack_req(op: Opcode) -> (i32, i32) {
    match op {
        I32Add => (2, -1),
        I32Mul => (2, -1),
        Drop => (1, -1),
        I32Const => (0, 1),
        LocalSet => (1, -1),
        LocalGet => (0, 1),
        LocalTee => (1, 0),
    }
}


#[derive(Debug, PartialEq, Eq, Hash)]
enum Operation {
    WithConst{constant: i32},
    WithLocal{other_local: i32},
}

impl Operation {
    fn gen_op<R: Rng + ?Sized>(rng: &mut R, num_locals: i32) -> Self {
        if rng.gen_bool(0.5) {
            // generate BinConst
            let constant = rng.gen_range(-10000..10000);
            Operation::WithConst{constant}
        } else {
            // generate BinTwo
            let other_local = rng.gen_range(1..=num_locals);
            Operation::WithLocal{other_local}
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Payload { 
    opcode: Opcode, input_local: i32, output_local: i32, op: Operation
}

impl Payload {
    fn gen_payloads<R: Rng + ?Sized>(rng: &mut R, num_locals: i32, num_payloads: i32) -> Vec<Self> {
        let mut out = vec![];

        let opcode_choices = [I32Add, I32Mul];
        let opcode_weights = [1, 1];
        let opcode_dist = WeightedIndex::new(&opcode_weights).expect("weights are hardcoded");

        for _ in 0..num_payloads {
            let opcode = opcode_choices[opcode_dist.sample(rng)];
            let input_local = rng.gen_range(1..=num_locals);
            let output_local = rng.gen_range(1..=num_locals);
            let op = Operation::gen_op(rng, num_locals);
            out.push(Payload{opcode, input_local, output_local, op});
        }
        out
    }
}

struct WatBuilder { 
    wat : String,
    opcode_counts : HashMap<Opcode, i32>
}

impl WatBuilder {
    // param 0 is loopcount; no one else should read or write it. param 1 is the "input" which 
    // everything depends on
    fn new() -> Self {
        let wat : String = r#"
        (module
          (func $main (export "main") (param i32 i32) (result i32)
        "#.to_string();
        let opcode_counts = HashMap::new();
        Self{wat, opcode_counts}
    }

    fn count_opcode(&mut self, op: Opcode) {
        *self.opcode_counts.entry(op).or_insert(0) += 1;
    }

    fn add_opcode(&mut self, op: Opcode, argument: Option<i32>) {
        self.wat.push_str(op.to_str());
        if let Some(i) = argument {
            self.wat += " ";
            self.wat += &i.to_string();
        }
        self.wat += "\n";
        self.count_opcode(op);
    }

    fn add_operation(&mut self, op: Operation) {
        match op {
            Operation::WithConst { constant } => self.add_opcode(I32Const, Some(constant)),
            Operation::WithLocal { other_local } => self.add_opcode(LocalGet, Some(other_local)),
        }
    }

    fn add_payload(&mut self, Payload { opcode, input_local, output_local, op }: Payload) {
        self.add_opcode(LocalGet, Some(input_local));
        self.add_operation(op);
        self.add_opcode(opcode, None);
        self.add_opcode(LocalSet, Some(output_local));        
    }

    fn push_str(&mut self, str: &str) {
        self.wat.push_str(str);
    }

    fn finish_wat(&mut self, num_locals: i32) {
        let finish_loop = r#"
        local.get 0
        i32.const 1 
        i32.sub 
        local.tee 0
        i32.const 0 
        i32.ne   
        br_if $loop 
        )
"#;
        self.wat.push_str(finish_loop);
        // return the sum of all locals, so that the code is not dead
        for i in 0..=num_locals {
            self.add_opcode(LocalGet, Some(i));
        }
        for _i in 0..num_locals - 1 {
            self.add_opcode(I32Add, None);
        }

        let wat_suffix = r#"
            return
            )
            (memory (export "memory") 0)
        )  
        "#;
        self.wat.push_str(wat_suffix);
    }

}

fn generate_wat<R: Rng + ?Sized>(rng: &mut R, set_payload_num: i32) -> WatBuilder {
    let mut builder = WatBuilder::new();

    // generate # of locals
    let min_locals = 3;
    let max_locals = 10;

    let num_locals = rng.gen_range(min_locals..=max_locals);
    // add that many locals to the wat 
    let mut locals_string = String::from("(local");
    for _ in 0..num_locals-1 { 
        locals_string.push_str(" i32");
    }
    locals_string.push_str(")\n");
    builder.push_str(&locals_string);
    builder.push_str("          (loop $loop\n");
    // initialize all other locals to the param
    for i in 1..=num_locals { 
        builder.add_opcode(LocalGet, Some(1));
        builder.add_opcode(LocalSet, Some(i));
    }

    // generate list of payloads
    let min_payloads = 100;
    let max_payloads = 200;
    // let num_payloads = rng.gen_range(min_payloads..=max_payloads);
    //for debugging?
    let num_payloads = set_payload_num;
    // dbg!(num_payloads);
    let payloads = Payload::gen_payloads(rng, num_locals, num_payloads);
    for payload in payloads {
        builder.add_payload(payload);
    }
    // append the suffix to finish out the wat
    builder.finish_wat(num_locals);
    builder
}

fn time_wat(imp1: i32, imp2: i32, num_loops: i32, wat: String) -> anyhow::Result<Duration> {
    let wasm = wabt::wat2wasm(wat).expect("it's a valid wat");
    let mut compiler = Cranelift::new();
    compiler.opt_level(CraneliftOptLevel::None);
    // dbg!(&compiler);
    let mut store = Store::new(compiler);
    // dbg!(store.engine());
    let module = Module::new(&store, &wasm)?;
    // dbg!(module.info());
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let main = instance.exports.get_function("main")?;

    let start = Instant::now();
    let result = main.call(&mut store, &[wasmer::Value::I32(num_loops), wasmer::Value::I32(imp1)])?;
    let result2 = main.call(&mut store, &[wasmer::Value::I32(num_loops), wasmer::Value::I32(imp2)])?;
    // let results : Vec<_> = (1..=1000)
    //   .map(|i|main.call(&mut store, &[wasmer::Value::I32(imp2)])).collect();
    let duration = start.elapsed();

    // dbg!(imp1, imp2, result, result2);
    // dbg!(results.clone().truncate(10));
    return Ok(duration)
}

fn generate_and_time_wat<R: Rng + ?Sized>(rng: &mut R, set_payload_num: i32) -> Duration {
    let wat_builder = generate_wat(rng, set_payload_num); 
    let mywat = wat_builder.wat;
    // let path = "gen_wat.txt";
    // let mut buffer = File::create(path)?;   
    // buffer.write_all(mywat.as_bytes())?;

    // println!("wat: {}", mywat);
    println!("counts: {:?}", wat_builder.opcode_counts);
    let imp1 = rng.gen();
    let imp2 = rng.gen();
    let duration = time_wat(imp1, imp2, 10_000, mywat).expect("wasmer exploded");
    return duration
}

fn myrng(loops: i32) -> u64 {
    let mut myrng = Lcg64XshRR32::default();
    for _ in 0..loops {myrng.advance()}
    myrng.state
}

fn watrng(loops: i32) -> anyhow::Result<(u64, Duration)> {
    let pcgwat = include_str!("pcg32.wat");
    let wasm = wabt::wat2wasm(pcgwat).expect("it's a valid wat");
    let mut compiler = Cranelift::new();
    compiler.opt_level(CraneliftOptLevel::None);
    let mut store = Store::new(compiler);
    let module = Module::new(&store, &wasm)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let instance = Instance::new(&mut store, &module, &import_object)?;
    let main = instance.exports.get_function("main")?;
    let start = Instant::now();
    let result = main.call(&mut store, &[wasmer::Value::I32(loops)])?;
    let duration = start.elapsed();
    match &result[0] {
        wasmer::Value::I64(ans) => return Ok((u64::from_be_bytes(ans.to_be_bytes()), duration)),
        other => panic!("wasmer returned {:?}", other)
    }
}

fn main() -> anyhow::Result<()>  {
    // // let mywat = include_str!("my_wat.wat");
    let mut rng = Pcg32::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7);
    // let payload_nums = [10];
    // let payload_nums = [100, 1_000, 10_000, 100_000, 1_000_000];
    // for payload_num in payload_nums { 
    let num_loops = 100;
    let payload_num = 100_000;
    for _ in 0..num_loops {
        let duration = generate_and_time_wat(&mut rng, payload_num);
        println!("num payloads: {:?} timing: {:?}", payload_num, duration);
    }

    // for i in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000] {
    //     let myval = myrng(i-1);
    //     let theirval = watrng(i).unwrap();
    //     println!("{} me {} them {:?} same {}", i, myval, theirval, myval == theirval.0);
    // }
    
    
    Ok(())
}

/* wasm-interp times:
15k -> 46ms 
49k -> 128ms
150k -> 338ms
494k -> 1090ms
*/