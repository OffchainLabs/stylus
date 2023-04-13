// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use std::{
    fs::{File, OpenOptions},
    io::Write,
    time::{Duration, Instant},
};

use arbutil::{format, operator::OperatorCode, Color};
use eyre::{eyre, Result};
use pricer::{fail, wasm, wat};
use prover::programs::{counter::CountingMachine, prelude::*};
use rand::{RngCore, rngs::ThreadRng};
use stylus::native::NativeInstance;
use std::arch::asm;

fn random_slice(mut rng: ThreadRng) -> Vec<u8> {
    let len = rand::random::<usize>() % 2048;
    let mut entropy = vec![0; len];
    rng.fill_bytes(&mut entropy);
    entropy
}

fn main() -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("data.csv")
        .unwrap();

    let mut start = Instant::now();
    let mut count = 0;

    println!("Pid {}", std::process::id());
    affinity::set_thread_affinity(&[1]).unwrap();
    let core = affinity::get_thread_affinity().unwrap();
    println!("Affinity {:?}", core);

    let rng = rand::thread_rng();

    for _ in 0..1 {
        let entropy = random_slice(rng.clone());

        for _ in 0..4 {
            trial(&entropy, &mut file)?;
        }
    }
    return Ok(());
    
    loop {
        let entropy = random_slice(rng.clone());
        trial(&entropy, &mut file)?;
        count += 1;

        if start.elapsed() >= Duration::new(150, 0) {
            let rate = count as f64 / 150.;
            println!("Exec: {:.1}", rate);
            //start = Instant::now();
            //count = 0;
            return Ok(())
        }
    }
}

fn trial(entropy: &[u8], file: &mut File) -> Result<()> {
    let module = wasm::random(&entropy, 1);
    if let Err(_error) = wasm::validate(&module) {
        return Ok(());
    }

    let mut timer = Instant::now();

    macro_rules! time {
        ($name:expr, $threshold:expr) => {{
            let elapsed = timer.elapsed();
            if elapsed.as_micros() > $threshold {
                println!("{} {}", $name.grey(), format::time(elapsed));
            }
            elapsed
        }};
    }

    let mut config = StylusConfig::version(1);
    config.debug.count_ops = true;
    config.debug.debug_funcs = true;
    config.pricing.ink_price = 100_00;
    config.start_ink = 500_000;
    config.costs = |_| 1;

    let mut native = match NativeInstance::from_bytes(&module, &config) {
        Ok(native) => native,
        Err(_) => return Ok(()),
    };
    let starter = native.get_start().expect("no start");
    time!("Init", 250_000);

    timer = Instant::now();
    let mut aux = 0_u32;
    //let before = unsafe { core::arch::x86_64::_rdtsc() };
    unsafe {
        libc::iopl(3);
        asm!("sti");
    };
    let before = unsafe { core::arch::x86_64::__rdtscp(&mut aux as *mut _) };
    let outcome = starter.call(&mut native.store).is_ok();
    //let cycles = unsafe { core::arch::x86_64::_rdtsc() - before };
    let cycles = unsafe { core::arch::x86_64::__rdtscp(&mut aux as *mut _) - before };
    unsafe { asm!("sti") };
    let elapsed = time!("Exec", 250_000);

    /*let cycles2 = unsafe {
        let before = core::arch::x86_64::__rdtscp(&mut aux as *mut _);
        starter.call(&mut native.store)?;
        core::arch::x86_64::_rdtsc() - before
    };*/

    let counts = native.operator_counts().unwrap();
    if counts.len() == 0 {
        return Ok(());
    }

    let outcome = outcome.then_some(1).unwrap_or_default();

    let nanos = elapsed.as_nanos();
    let ratio = cycles as f64 / nanos as f64;

    write!(file, "{cycles} {nanos} {ratio:.2} {outcome} ")?;
    for (op, count) in counts {
        write!(file, "{} {} ", op.0, count)?;
    }
    writeln!(file)?;
    Ok(())
}
