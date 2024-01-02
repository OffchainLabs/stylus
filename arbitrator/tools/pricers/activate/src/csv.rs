// Copyright 2022-2024, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::Trial;
use eyre::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

pub fn csv(path: PathBuf, field: String) -> Result<()> {
    let file = BufReader::new(File::open(path)?);

    println!("{field},wasm_len,funcs,code,data,mem_size");
    //println!("init,wasm_len,funcs,code,data,mem_size");
    //println!("asm,wasm_len,funcs,code,data,mem_size");
    //println!("mod,wasm_len,funcs,code,data,mem_size");
    //println!("parse,wasm_len");

    for line in file.lines() {
        let t: Trial = line?.parse()?;
        let i = t.info;

        let time = match field.as_str() {
            "parse" => t.parse_time,
            "mod" => t.module_time,
            "hash" => t.hash_time,
            "brotli" => t.brotli_time,
            "asm" => t.asm_time,
            "init" => t.init_time,
            x => panic!("unknown field {field}"),
        };

        println!(
            "{},{},{},{},{},{}",
            time.as_micros(),
            t.wasm_len,
            i.funcs,
            i.code,
            i.data,
            i.mem_size
        );
    }
    Ok(())
}
