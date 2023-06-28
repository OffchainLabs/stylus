// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbutil::{color, operator::OperatorCode};
use num::{Saturating, Zero};
use rand::{distributions::Standard, prelude::Distribution, Rng};
use std::ops::Add;

pub fn random_vec<T>(len: usize) -> Vec<T>
where
    Standard: Distribution<T>,
{
    let mut rng = rand::thread_rng();
    let mut entropy = Vec::with_capacity(len);
    for _ in 0..len {
        entropy.push(rng.gen())
    }
    entropy
}

pub fn _abs_diff(a: usize, b: usize) -> usize {
    let max = a.max(b);
    let min = a.min(b);
    max - min
}

pub trait SaturatingSum: Iterator {
    fn saturating_sum(self) -> Self::Item;
}

impl<I: Iterator> SaturatingSum for I
where
    Self::Item: Add<Output = Self::Item> + Saturating + Zero + Copy,
{
    fn saturating_sum(self) -> Self::Item {
        self.fold(Self::Item::zero(), |acc, x| acc.saturating_add(x))
    }
}

pub fn format_nanos(nanos: usize) -> String {
    arbutil::format::time(std::time::Duration::from_nanos(nanos as u64))
}

pub fn op_used(op: OperatorCode) -> bool {
    match op.0 {
        0x0..=0x5 => true,
        0xb..=0x11 => true,
        0x1a..=0x1b => true,
        0x20..=0x24 => true,
        0x28..=0x29 => true,
        0x2c..=0x37 => true,
        0x3a..=0x42 => true,
        0x45..=0x5a => true,
        0x67..=0x8a => true,
        0xa7 => true,
        0xac..=0xad => true,
        0xc0..=0xc4 => true,
        0xfc0a..=0xfc0b => true,
        _ => false,
    }
}

pub fn op_color(op: OperatorCode) -> &'static str {
    if !op_used(op) {
        return color::RED;
    }
    match op.0 {
        0x00 | 0x02..=0x11 => color::PINK,        // control flow
        0x1a | 0x1c..=0x24 => color::MINT,        // variable
        0x28..=0x40 => color::YELLOW,             // memory
        0x01 | 0x1b | 0x41..=0xc4 => color::BLUE, // numeric
        0xfc0a..=0xfc0b => color::YELLOW,         // bulk memory
        _ => color::GREY,
    }
}
