// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbutil::{operator::OperatorCode, Color};
use eyre::Result;
use indexmap::IndexSet;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use wasmer::wasmparser::Operator;
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader},
};

fn main() -> Result<()> {
    let file = OpenOptions::new()
        .create(false)
        .read(true)
        .write(false)
        .append(false)
        .open("data.csv")
        .unwrap();
    let reader = BufReader::new(file);

    const OP_COUNT: usize = 185;

    let mut ops = IndexSet::new();
    let mut terms = vec![];

    let mut cycles = vec![];
    let mut status = vec![];

    for (i, line) in reader.lines().enumerate()
    /*.take(1_000_000)*/
    {
        let line = line?;
        let data: Vec<_> = line.split(' ').collect();
        let time: usize = data[0].parse()?;
        let success: usize = data[1].parse()?;

        if i % 100_000 == 0 {
            println!("Line: {i} {} {}", ops.len(), line.grey());
        }
        
        if success == 0 {
                //continue;
        }

        if i > 2_000_000 {
            break;
        }

        let mut counts = [0.; OP_COUNT];

        for i in (2..data.len() - 1).step_by(2) {
            let op = OperatorCode(data[i].parse()?);
            let count: usize = data[i + 1].parse()?;

            let index = ops.insert_full(op).0;
            counts[index] += count as f64;
        }

        /*if let Some(_) = ops.get_index_of(&OperatorCode(0x0e)) {
            //if counts[index] != 0. {
                ops.remove(&OperatorCode(0x0e));
                continue;
            //}
        }*/

        cycles.push(time as f64);
        status.push(success as f64);
        terms.push(counts);
    }

    println!("Done: {} {}", terms.len(), ops.len());

    for _ in 0..10_000 {
        terms.pop();
        cycles.pop();
        status.pop();
    }

    let op_kinds = ops.len();

    let mut data = vec![
        ("Cycles".to_string(), cycles.clone()),
        ("Status".to_string(), status.clone()),
    ];
    for kind in 0..op_kinds {
        let op = ops.get_index(kind).unwrap();
        let name = op.to_string();
        data.push((name, vec![]));
    }
    for vars in terms {
        for i in 0..op_kinds {
            data[i + 2].1.push(vars[i]);
        }
    }

    let columns: Vec<_> = data.iter().skip(1).map(|x| x.0.to_owned()).collect();
    //let columns: Vec<_> = ops.iter().map(ToString::to_string).collect();

    let data = RegressionDataBuilder::new().build_from(data)?;
    let model = FormulaRegressionBuilder::new()
        .data(&data)
        .data_columns("Cycles", columns)
        .fit()?;

    let params = model.parameters();
    let errors = model.se();
    println!("Intercept: {:.2}", params[0]);

    let status = params[1];
    println!("Status: {:.2}", status);

    let mut ci_avg = 0.;
    for i in 0..op_kinds {
        let op = ops.get_index(i).unwrap();
        let param = params[i + 2];
        let se = errors[i + 2];
        let conf = 1.96; // df → ∞
        let ci = format!("∈ ({:.2}, {:.2})", param - se * conf, param + se * conf);
        if param + se * conf >= 0. {
            println!("{op} {:.2}\t{}", param, ci.grey());
        } else {
            println!("{op} {:.2}\t{}", param, ci.red());
        }
        ci_avg += 2. * se * conf;
    }

    println!("R^2 = {}", model.rsquared());
    println!("Ci = {}", ci_avg / op_kinds as f64);

    Ok(())
}
