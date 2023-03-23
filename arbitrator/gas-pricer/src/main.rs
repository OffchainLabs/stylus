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
}
use Opcode::*;

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

fn generate_wat<R: Rng + ?Sized>(rng: &mut R) -> String {
    // skeleton is fixed
    let mut wat : String = r#"
    (module
      (func $main (export "main")
    "#.to_string();
    // this will be appended to the wat at the end
    let wat_suffix = r#"
      )
      (memory (export "memory") 0)
      (start 0)
    )  
    "#;
    // generate # of locals
    let min_locals = 1;
    let max_locals = 10;

    let num_locals = rng.gen_range(min_locals..=max_locals);
    // add that many locals to the wat 
    let mut locals_string = String::from("(local");
    for _ in 0..num_locals { 
        locals_string.push_str(" i32")
    }
    locals_string.push_str(")\n");
    wat.push_str(&locals_string);
    // initialize local i to the integer i 
    for i in 1..=num_locals { 
        let mut init_string = "i32.const ".to_string() + &i.to_string();
        init_string += "\nlocal.set "; 
        init_string += &(i-1).to_string();
        init_string += "\n";
        wat.push_str(&init_string);
    }

    // generate list of payloads
    let min_payloads = 10;
    let max_payloads = 20;
    let payload_choices = [I32Add, I32Mul, Drop];
    let payload_weights = [5, 5, 1];
    let payload_dist = WeightedIndex::new(&payload_weights).expect("weights are hardcoded");
    
    let num_payloads = rng.gen_range(min_payloads..=max_payloads);
    let mut payloads = vec![];
    for _ in 0..num_payloads {
        let payload = payload_choices[payload_dist.sample(rng)];
        payloads.push(payload);
    }

    let mut stack_depth = 0;
    let stack_increase_choices = [LocalGet, I32Const]; 
    let stack_increase_weights = [1, 1];
    let stack_increase_dist = WeightedIndex::new(&stack_increase_weights).expect("weights are hardcoded");
    // push payloads into string, keeping track of depth of stack 
    for payload in payloads {
        let (stack_req, stack_change) = opcode_stack_req(payload);
        while stack_depth < stack_req {
            // we need to add an opcode which increases the size of the stack 
            let increase_op = stack_increase_choices[stack_increase_dist.sample(rng)];
            match increase_op {
                LocalGet => {
                    let local_to_get = rng.gen_range(0..num_locals);
                    wat += "local.get "; 
                    wat += &local_to_get.to_string();
                    wat += "\n";
                }, 
                other => {
                    assert_eq!(other, I32Const); 
                    let const_val = rng.gen_range(-100..100);
                    wat += "i32.const ";
                    wat += &const_val.to_string();
                    wat += "\n"
                },   
            }
            let (_increase_req, increase_amt) = opcode_stack_req(increase_op);
            
            stack_depth += increase_amt;
            dbg!(increase_op, stack_depth);
        }
        wat += payload.to_str();
        wat += "\n";
        stack_depth += stack_change;
        dbg!(payload, stack_depth); 
    }

    // append the suffix to finish out the wat
    wat.push_str(wat_suffix);
    wat

}

fn main() -> anyhow::Result<()>  {
    // let mywat = include_str!("my_wat.wat");
    let mut rng = Pcg32::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7);
    let mywat = generate_wat(&mut rng); 
    println!("wat: {}", mywat);
    let mywasm = wabt::wat2wasm(mywat).expect("it's a valid wat");
    let mut store = Store::default();
    let module = Module::new(&store, &mywasm)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let add_one = instance.exports.get_function("main")?;
    let result = add_one.call(&mut store, &[])?;

    println!("res: {:?}", result);


    Ok(())
}
