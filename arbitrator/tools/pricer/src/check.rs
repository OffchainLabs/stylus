// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbutil::{format, operator::OperatorCode};
use eyre::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    iter::once,
    path::Path,
    str::FromStr,
};

use crate::{
    model::{Model, Trial, OP_COUNT},
    util::{self, op_used},
};

pub fn check(path: &Path) -> Result<()> {
    use OperatorCode as Op;

    let mut model = Model::default();

    macro_rules! set_range {
        ($range:expr, $value:expr) => {
            for op in $range {
                model.set(Op(op), $value); 
            }
        };
    }

    set_range!(0x45..=0x4f, 0.341); // i32 comparisons
    set_range!(0x50..=0x5a, 0.183); // i64 comparisons
    set_range!(0x67..=0x69, 0.240); // i32 bit counters
    set_range!(0x79..=0x7b, 0.240); // i64 bit counters

    set_range!(0x6a..=0x6b, 0.1); // fast i32 bin ops
    set_range!(0x71..=0x78, 0.1); // fast i32 bin ops
    set_range!(0x7c..=0x7d, 0.1); // fast i64 bin ops
    set_range!(0x83..=0x8a, 0.1); // fast i64 bin ops

    set_range!(0xc0..=0xc1, 0.1); // i32 extensions
    set_range!(0xac..=0xad, 0.1); // i64 extensions
    set_range!(0xc2..=0xc4, 0.1); // i64 extensions
    set_range!(0xa7..=0xa7, 0.1); // i64 extensions

    set_range!(0x6d..=0x70, 0.1); // i32 divisions
    set_range!(0x7f..=0x82, 0.1); // i64 divisions

    set_range!(0x0c..=0x0d, 0.58); // branching

    set_range!(0x41..=0x41, 0.1); // i32 const
    set_range!(0x42..=0x42, 0.1); // i64 const

    set_range!(0x6c..=0x6c, 0.1); // i32 mul
    set_range!(0x7e..=0x7e, 0.1); // i64 mul

    model.weights[OP_COUNT + 0] = 0.;
    model.weights[OP_COUNT + 1] = 757.62; // grace
    model.weights[OP_COUNT + 2] = 18388.59; // traps

    for op in 0x41..=0xc4 {
        let op = Op(op);
        if op_used(op) {
            if model.weights[op.seq()] == 0. {
                panic!("Not set {}", op);
            }
        }
    }

    let file = BufReader::new(File::open(path)?);

    let mut above = 0.;
    let mut below = 0.;
    let mut above_percent = 0.;
    let mut below_percent = 0.;
    let mut above_count = 0;
    let mut below_count = 0;
    for line in file.lines() {
        let trial = Trial::from_str(&line?)?;
        let (error, percent) = model.error(&trial);
        if error > 0. {
            above += error;
            above_percent += percent;
            above_count += 1;
        } else {
            below -= error;
            below_percent -= percent;
            below_count += 1;
        }
    }
    println!(
        "Error {} {}",
        util::format_nanos(above as usize / above_count),
        util::format_nanos(below as usize / below_count),
    );
    println!(
        "Error {:.2}% {:.2}%",
        above_percent / above_count as f64,
        below_percent / below_count as f64
    );

    Ok(())
}
