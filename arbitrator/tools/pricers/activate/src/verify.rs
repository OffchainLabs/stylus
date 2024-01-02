// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use activate::{util, Trial};
use arbutil::{color::Color, format};
use eyre::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::Duration,
};

pub fn verify(path: &Path) -> Result<()> {
    verify_impl(path, "parse", |trial| {
        let pred = trial.pred_parse_us() as f64;
        let actual = trial.parse_time.as_micros() as f64;
        (pred, actual)
    })?;

    verify_impl(path, "module", |trial| {
        let pred = trial.pred_module_us() as f64;
        let actual = trial.module_time.as_micros() as f64;
        (pred, actual)
    })?;

    verify_impl(path, "asm", |trial| {
        let pred = trial.pred_asm_us() as f64;
        let actual = trial.asm_time.as_micros() as f64;
        (pred, actual)
    })?;

    verify_impl(path, "brotli", |trial| {
        let pred = trial.pred_brotli_us() as f64;
        let actual = trial.brotli_time.as_micros() as f64;
        (pred, actual)
    })?;

    verify_impl(path, "hash", |trial| {
        let pred = trial.pred_hash_us() as f64;
        let actual = trial.hash_time.as_micros() as f64;
        (pred, actual)
    })
}

pub fn verify_impl(path: &Path, name: &str, apply: fn(Trial) -> (f64, f64)) -> Result<()> {
    let file = BufReader::new(File::open(path)?);

    let mut high: f64 = f64::MIN;
    let mut avg: f64 = 0.;
    let mut low: f64 = f64::MAX;
    let mut count = 0;

    let mut naive: u64 = 0;
    let mut model: u64 = 0;
    let mut load: u64 = 0;

    for line in file.lines() {
        let trial: Trial = line?.parse()?;

        let (pred, actual) = apply(trial);

        high = high.max(actual - pred);
        low = low.min(actual - pred);
        avg += 100. * (actual - pred) / actual;
        count += 1;

        naive = naive.max(actual as u64);
        model += pred as u64;
        load += actual as u64;

        if actual > pred {
            println!("pred {} {}", actual.red(), pred.red());
        }
    }

    avg = avg / count as f64;
    model = model / count;
    load = load / count;

    println!("{name} prediction");
    println!("high:  {high}");
    println!("low:   {low}");
    println!("avg:   {:.1}%", avg);
    println!("count: {count}");

    println!(
        "naive: {}",
        format::gas(util::gas(Duration::from_micros(naive), 2.))
    );
    println!(
        "model: {}",
        format::gas(util::gas(Duration::from_micros(model), 2.))
    );
    if model < naive {
        println!(
            "saved: {} ^.^",
            format::gas(util::gas(Duration::from_micros(naive - model), 2.))
        );
    }
    println!(
        "oppt:  {}",
        format::gas(util::gas(Duration::from_micros(model - load), 2.))
    );
    println!();
    Ok(())
}
