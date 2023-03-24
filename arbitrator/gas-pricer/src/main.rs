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
use std::time::Duration;
use std::time::Instant;

use rand::prelude::*;
use rand::Rng;
use rand::distributions::WeightedIndex;
use rand_pcg::Pcg32;
use wasmer::{Store, Module, Instance, Value, imports};
use wasmer::FunctionEnv;
use anyhow;

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

struct WatBuilder { 
    wat : String,
    opcode_counts : HashMap<Opcode, i32>
}

impl WatBuilder {
    fn new() -> Self {
        let wat : String = r#"
        (module
          (func $main (export "main") (param i32) (result i32)
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

    fn push_str(&mut self, str: &str) {
        self.wat.push_str(str);
    }

    fn finish_wat(&mut self) {
        let wat_suffix = r#"
            return
            )
            (memory (export "memory") 0)
        )  
        "#;
        self.wat.push_str(wat_suffix);
    }

}

fn generate_wat<R: Rng + ?Sized>(rng: &mut R) -> WatBuilder {
    let mut builder = WatBuilder::new();

    // generate # of locals
    let min_locals = 3;
    let max_locals = 10;

    let num_locals = rng.gen_range(min_locals..=max_locals);
    // add that many locals to the wat 
    let mut locals_string = String::from("(local");
    for _ in 0..num_locals { 
        locals_string.push_str(" i32");
    }
    locals_string.push_str(")\n");
    builder.push_str(&locals_string);
    // initialize local i to the integer i; note local 0 is the param
    for i in 1..=num_locals { 
        builder.add_opcode(I32Const, Some(i));
        builder.add_opcode(LocalSet, Some(i));
    }

    // generate list of payloads
    let min_payloads = 150;
    let max_payloads = 200;
    let num_payloads = rng.gen_range(min_payloads..=max_payloads);

    let payload_choices = [LocalSet, I32Add, I32Mul];
    let payload_weights = [8, 5, 5];
    let payload_dist = WeightedIndex::new(&payload_weights).expect("weights are hardcoded");
    
    dbg!(num_payloads);
    let mut payloads = vec![];
    for _ in 0..num_payloads {
        let payload = payload_choices[payload_dist.sample(rng)];
        payloads.push(payload);
    }

    // just for debugging
    let payloads_to_keep = 106;
    payloads.truncate(payloads_to_keep);

    // get the param to main, so everything depends on it and can't be removed 
    // by constant folding
    builder.add_opcode(LocalGet, Some(0));
    let mut stack_depth = 1;

    let stack_increase_choices = [LocalGet, I32Const]; 
    let stack_increase_weights = [10, 1];
    let stack_increase_dist = WeightedIndex::new(&stack_increase_weights).expect("weights are hardcoded");
    // push payloads into string, keeping track of depth of stack 
    for payload in payloads {
        let (stack_req, stack_change) = opcode_stack_req(payload);
        while stack_depth < stack_req {
            // we need to add an opcode which increases the size of the stack 
            let increase_op = stack_increase_choices[stack_increase_dist.sample(rng)];
            match increase_op {
                LocalGet => {
                    let local_to_get = rng.gen_range(0..=num_locals);
                    builder.add_opcode(LocalGet, Some(local_to_get));
                }, 
                other => {
                    assert_eq!(other, I32Const); 
                    let const_val = rng.gen_range(-100..100);
                    builder.add_opcode(I32Const, Some(const_val));
                },   
            }
            let (_increase_req, increase_amt) = opcode_stack_req(increase_op);
            
            stack_depth += increase_amt;
            // dbg!(increase_op, stack_depth);
        }
        let payload_arg = if payload == LocalSet {
            Some(rng.gen_range(0..=num_locals))
        } else {None};
        builder.add_opcode(payload, payload_arg);
        stack_depth += stack_change;
        // dbg!(payload, stack_depth); 
    }

    // take the produce of all the remaining stack values with all locals
    for _ in 0..stack_depth-1 {
        builder.add_opcode(I32Mul, None);
    }

    // return the product of all locals, so that the code is not dead
    for i in 0..=num_locals {
        builder.add_opcode(LocalGet, Some(i));
    }
    for _i in 0..num_locals - 1 {
        builder.add_opcode(I32Mul, None);
    }
    // append the suffix to finish out the wat
    builder.finish_wat();
    builder
}

fn time_wat(wat: String) -> anyhow::Result<Duration> {
    let wasm = wabt::wat2wasm(wat).expect("it's a valid wat");
    let mut store = Store::default();
    let module = Module::new(&store, &wasm)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let main = instance.exports.get_function("main")?;

    let start = Instant::now();
    let result = main.call(&mut store, &[wasmer::Value::I32(57)])?;
    let duration = start.elapsed();
    let result2 = main.call(&mut store, &[wasmer::Value::I32(58)])?;
    dbg!(result, result2);
    return Ok(duration)
}

fn main() -> anyhow::Result<()>  {
    // let mywat = include_str!("my_wat.wat");
    let mut rng = Pcg32::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7);
    let wat_builder = generate_wat(&mut rng); 
    let mywat = wat_builder.wat;
    println!("wat: {}", mywat);
    println!("counts: {:?}", wat_builder.opcode_counts);
    let duration = time_wat(mywat);
    // TODO: 1) time execution of the wasm 2) count opcodes put in the wasm
    println!("timing: {:?}", duration);
    Ok(())
}

/* times (fastest of 3)
180 -> 93u
1097 -> 92
10978 -> 86
*/