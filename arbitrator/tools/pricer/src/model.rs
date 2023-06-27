// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::util::{self, SaturatingSum};
use arbutil::{color::Color, operator::OperatorCode};
use eyre::{bail, ErrReport, Result};
use rand::Rng;
use std::{
    convert::TryInto,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, Write},
    path::Path,
    str::FromStr,
};

type Weights = [f64; OP_COUNT + 3];
type Ops = [usize; OP_COUNT];

pub const OP_COUNT: usize = 529;

pub struct Trial {
    nanos: usize,
    ops: Ops,
    status: bool,
}

impl FromStr for Trial {
    type Err = ErrReport;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let data: Vec<_> = s.trim().split(' ').collect();
        let mut ops = [0; OP_COUNT];

        macro_rules! parse {
            ($i:expr) => {{
                let Some(data) = data.get($i) else {
                                                        // happens when recording new data
                                                        bail!("incomplete line: {s}");
                                                    };
                match data.parse() {
                    Ok(data) => data,
                    Err(err) => bail!("{err}: {s}"),
                }
            }};
        }

        let nanos = parse!(0);
        let status = parse!(1);
        for i in (2..data.len() - 1).step_by(2) {
            let op = OperatorCode(parse!(i));
            ops[op.seq()] = parse!(i + 1);
        }
        Ok(Trial { nanos, status, ops })
    }
}

impl Trial {
    pub fn print(&self) {
        println!("Trial: {} {}", self.nanos, self.status);
        for (op, count) in self.ops.iter().enumerate().filter(|x| *x.1 != 0) {
            let op = OperatorCode::from_seq(op);
            let line = format!("{} {:.2} ", op, count);
            match util::op_used(op) {
                true => println!("{}", line),
                false => println!("{}", line.red()),
            }
        }
    }
}

#[derive(Clone)]
pub struct Model {
    pub weights: Weights,
    fitness: Option<usize>,
}

impl Model {
    fn new(weights: Weights) -> Self {
        let fitness = None;
        Self { weights, fitness }
    }

    pub fn set(&mut self, op: OperatorCode, weight: f64) {
        self.weights[op.seq()] = weight;
    }

    /// Assign random weights
    fn random() -> Self {
        let mut data: Vec<f64> = util::random_vec(OP_COUNT + 3); // between 0 and 1
        for i in 0..3 {
            data[OP_COUNT + i] = rand::random::<f64>() * 1000.;
        }
        Self::new(data.try_into().unwrap())
    }

    /// Adjust a weight randomly
    fn mutate(&mut self) {
        let mut rng = rand::thread_rng();
        let coef = rng.gen_range(0.75..1.25);
        let term = rng.gen_range(-0.1..0.1);

        let point = match rand::random() {
            true => rand::random::<usize>() % 188,
            false => rand::random::<usize>() % 3 + OP_COUNT,
        };
        let mut update = coef * self.weights[point];
        update = update + term;
        if update < 0. {
            update = 0.;
        }

        if update.is_finite() {
            self.weights[point] = update;
            self.fitness = None;
        }
    }

    fn cross_over(&self, other: &Self) -> Self {
        let point = rand::random::<usize>() % self.weights.len();
        let mut data = other.weights;
        unsafe { std::ptr::copy_nonoverlapping(self.weights.as_ptr(), data.as_mut_ptr(), point) }

        Model::new(data)
    }

    pub fn eval(&self, trial: &Trial) -> f64 {
        let fixed_weight = self.weights[OP_COUNT + 0];
        let success_weight = self.weights[OP_COUNT + 1];
        let failure_weight = self.weights[OP_COUNT + 2];

        let mut predict = fixed_weight;
        for (count, weight) in trial.ops.iter().zip(self.weights) {
            predict += *count as f64 * weight;
        }
        predict += match trial.status {
            true => success_weight,
            false => failure_weight,
        };

        // TODO: bias
        (predict - trial.nanos as f64).abs()
    }

    pub fn error(&self, trial: &Trial) -> (f64, f64) {
        let fixed_weight = self.weights[OP_COUNT + 0];
        let success_weight = self.weights[OP_COUNT + 1];
        let failure_weight = self.weights[OP_COUNT + 2];

        let mut predict = fixed_weight;
        for (count, weight) in trial.ops.iter().zip(self.weights) {
            predict += *count as f64 * weight;
        }
        predict += match trial.status {
            true => success_weight,
            false => failure_weight,
        };

        // TODO: bias
        let error = predict - trial.nanos as f64;
        (error, 100. * error / trial.nanos as f64)
    }

    pub fn print(&self) {
        println!("{}", "Model:".grey());
        let mut col = 0;
        for (op, nanos) in self.weights.iter().enumerate().take(OP_COUNT) {
            let op = OperatorCode::from_seq(op);
            if util::op_used(op) {
                let entry = format!("{:02x} {:12} {:.2} ", op.0, op, nanos);
                let entry = format!("{entry:23}");
                print!("{}", entry.color(util::op_color(op)));

                col += 1;
                if col % 5 == 0 {
                    println!();
                }
            }
        }
        println!();
        let greyln = |x: String| println!("{}", x.grey());
        greyln(format!("Fixed {:.2}", self.weights[OP_COUNT + 0]).grey());
        greyln(format!("Grace {:.2}", self.weights[OP_COUNT + 1]).grey());
        greyln(format!("Traps {:.2}", self.weights[OP_COUNT + 2]).grey());

        let avg = |x: usize, y| {
            let x = OperatorCode(x).seq();
            let y = OperatorCode(y).seq();
            let sum: f64 = (x..=y).map(|i| self.weights[i]).sum();
            sum / (y - x + 1) as f64
        };

        println!("I32 Cmp: {:.3}", avg(0x45, 0x4f));
        println!("I64 Cmp: {:.3}", avg(0x50, 0x5a));
        println!("I32 Fast Bin: {:.3}", avg(0x6a, 0x6b));
        println!("I64 Fast Bin: {:.3}", avg(0x7c, 0x7d));
    }

    fn print_data(&self) -> String {
        let mut text = String::new();
        for nanos in &self.weights {
            text += &format!("{nanos} ");
        }
        text
    }
}

impl FromStr for Model {
    type Err = ErrReport;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let data: Vec<_> = s.trim().split(' ').collect();
        let mut weights = [0.; OP_COUNT + 3];

        macro_rules! parse {
            ($i:expr) => {
                match data[$i].parse() {
                    Ok(data) => data,
                    Err(err) => bail!("{s}: {err}"),
                }
            };
        }

        for i in 0..weights.len() {
            weights[i] = parse!(i);
        }

        let fitness = None;
        Ok(Self { weights, fitness })
    }
}

impl Default for Model {
    fn default() -> Self {
        Model {
            weights: [0.; OP_COUNT + 3],
            fitness: None,
        }
    }
}

struct Pop {
    models: Vec<Model>,
    best: Model,
}

impl Pop {
    fn new(size: usize) -> Pop {
        let mut models = vec![];
        for _ in 0..size {
            models.push(Model::random());
        }
        Pop {
            models,
            best: Model::random(),
        }
    }

    fn len(&self) -> usize {
        self.models.len()
    }

    fn sort(&mut self) {
        self.models.sort_by_key(|m| m.fitness);
    }

    fn stats(&self) -> (usize, usize) {
        let best = self.models[0].fitness.unwrap();
        let total = self
            .models
            .iter()
            .map(|m| m.fitness.unwrap())
            .saturating_sum();
        let avg = total / self.models.len();
        (avg, best)
    }

    fn select(&self) -> &Model {
        let mut rng = rand::thread_rng();
        let i = rng.gen_range(0..self.models.len() / 2);
        &self.models[i]
    }
}

struct Feed {
    file: BufReader<File>,
}

impl Feed {
    fn new(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(&path)?);
        Ok(Self { file })
    }

    fn batch(&mut self, size: usize) -> Result<Vec<Trial>> {
        let mut trials = vec![];
        for _ in 0..size {
            let line = self.read_line()?;
            trials.push(Trial::from_str(&line)?);
        }
        Ok(trials)
    }

    fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        if self.file.read_line(&mut line)? == 0 {
            self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
            println!("restarting feed");

            line.clear();
            self.file.read_line(&mut line)?;
        }
        Ok(line)
    }
}

pub fn model(path: &Path, output: &Path) -> Result<()> {
    let mut feed = Feed::new(path)?;
    let mut save = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(output)?;

    let mut pop = Pop::new(256);

    for gen in 0.. {
        let trials = feed.batch(1024)?;
        let trial_time = trials.iter().map(|x| x.nanos).saturating_sum();
        let trial_time = trial_time / trials.len();

        for model in &mut pop.models {
            let mut total = 0.;
            for trial in &trials {
                total += model.eval(trial);
            }
            model.fitness = Some(total as usize / trials.len());
        }

        let curr_best = pop.best.fitness.unwrap_or(usize::MAX);

        pop.sort();

        let (pop_fitness, pop_best) = pop.stats();
        let percent = 100. * pop_best as f64 / trial_time as f64;
        println!(
            "Gen {gen} {percent:.2}% {} {} {}",
            util::format_nanos(curr_best),
            util::format_nanos(pop_best),
            util::format_nanos(pop_fitness)
        );

        if pop_best < curr_best {
            pop.best = pop.models[0].clone();
        }
        if pop_best < curr_best || gen % 100 == 0 {
            writeln!(&mut save, "{}", pop.models[0].print_data())?;
            save.flush()?;
        }

        let mut new_pop = vec![];
        while new_pop.len() < pop.len() {
            let a = pop.select();
            let b = pop.select();

            let mut child = a.cross_over(b);
            child.mutate();
            new_pop.push(child);
        }

        pop.models = new_pop;
    }

    unreachable!()
}
