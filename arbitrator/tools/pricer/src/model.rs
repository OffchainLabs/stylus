// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::util::{self, SaturatingSum};
use arbutil::{color::Color, operator::OperatorCode};
use eyre::{bail, ErrReport, Result};
use rand::Rng;
use std::{
    collections::VecDeque,
    convert::TryInto,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    str::FromStr,
};

type Groups = [usize; OP_COUNT];
type Weights = [f64; OP_COUNT + 3];
type Ops = [usize; OP_COUNT];

//pub const OP_COUNT: usize = 529;
pub const OP_COUNT: usize = 256;

#[derive(Clone)]
pub struct Trial {
    nanos: usize,
    ops: Ops,
    status: bool,
    bucket: usize,
}

impl FromStr for Trial {
    type Err = ErrReport;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let data: Vec<_> = s.trim().split(' ').collect();
        let mut ops = [0; OP_COUNT];

        macro_rules! parse {
            ($i:expr) => {{
                let data = match data.get($i) {
                    Some(data) => data,
                    None => bail!("incomplete line: {s}"), // happens when recording new data
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
        Ok(Trial {
            nanos,
            status,
            ops,
            bucket: 0,
        })
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

    pub fn get(&self, op: OperatorCode) -> f64 {
        self.weights[op.seq()]
    }

    pub fn set(&mut self, op: OperatorCode, weight: f64) {
        self.weights[op.seq()] = weight;
    }

    /// Assign random weights
    fn random(groups: &Groups) -> Self {
        //let mut data: Vec<f64> = util::random_vec(OP_COUNT + 3); // between 0 and 1
        let mut data = [0.; OP_COUNT + 3];
        for i in 0..OP_COUNT {
            data[groups[i]] = rand::random();
        }
        for i in 0..3 {
            data[OP_COUNT + i] = rand::random::<f64>() * 1000.;
        }
        Self::new(data.try_into().unwrap())
    }

    /// Adjust a weight randomly
    fn mutate(&mut self, groups: &Groups) {
        let mut rng = rand::thread_rng();
        let coef = rng.gen_range(0.75..1.25);
        let term = rng.gen_range(-0.001..0.001);

        let point = match rand::random() {
            true => groups[rand::random::<usize>() % 188],
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

    pub fn eval(&self, trial: &Trial, groups: &Groups) -> f64 {
        let fixed_weight = self.weights[OP_COUNT + 0];
        let success_weight = self.weights[OP_COUNT + 1];
        let failure_weight = self.weights[OP_COUNT + 2];

        let mut predict = fixed_weight;
        for (count, weight) in trial.ops.iter().zip(self.weights) {
            predict += *count as f64 * weight;
        }
        for (index, count) in trial.ops.iter().enumerate() {
            let op = groups[index];
            let weight = self.weights[op];
            predict += *count as f64 * weight;
        }
        predict += match trial.status {
            true => success_weight,
            false => failure_weight,
        };

        // we impose a large penalty to under-predicting costs
        let mut fitness = predict - trial.nanos as f64;
        if fitness < 0. {
            fitness *= -100.
        }
        fitness
    }

    pub fn error(&self, trial: &Trial, groups: &Groups) -> (f64, f64) {
        let fixed_weight = self.weights[OP_COUNT + 0];
        let success_weight = self.weights[OP_COUNT + 1];
        let failure_weight = self.weights[OP_COUNT + 2];

        let mut predict = fixed_weight;
        for (index, count) in trial.ops.iter().enumerate() {
            let op = groups[index];
            let weight = self.weights[op];
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

    pub fn full_eval(&self, path: &Path, groups: &Groups) -> Result<()> {
        let file = BufReader::new(File::open(path)?);
        self.print(groups);

        let threshold = 90.;
        let mut issues = 0;
        let mut trail_count = 0;

        let mut above = 0.;
        let mut below = 0.;
        let mut percent_above = 0.;
        let mut percent_below = 0.;
        let mut count_above = 0;
        let mut count_below = 0;
        let mut worst_above: f64 = 0.;
        let mut worst_below: f64 = 0.;
        for line in file.lines() {
            let trial = Trial::from_str(&line?)?;
            let (error, percent) = self.error(&trial, &groups);
            if error > 0. {
                above += error;
                percent_above += percent;
                count_above += 1;
                worst_above = worst_above.max(percent);
            } else {
                below -= error;
                percent_below -= percent;
                count_below += 1;
                worst_below = worst_below.max(percent.abs());
                if -percent >= threshold {
                    issues += 1;
                }
            }
            trail_count += 1;
        }
        println!(
            "Error {} {}",
            util::format_nanos(above as usize / count_above),
            util::format_nanos(below as usize / count_below),
        );
        println!(
            "Error {:.2}% {:.2}% {:.2}% {:.2}% {issues}/{trail_count}",
            percent_above / count_above as f64,
            percent_below / count_below as f64,
            worst_above,
            worst_below,
        );
        Ok(())
    }

    pub fn print(&self, groups: &Groups) {
        let gas = 0.064;

        let f = |x: f64| {
            let mut s = format!("{x:.4}");
            if let Some(small) = s.strip_prefix("0.") {
                s = format!(".{small}");
            }
            s
        };

        let mut col = 0;
        for index in 0..OP_COUNT {
            let op = OperatorCode::from_seq(index);
            let nanos = self.weights[groups[index]];
            let group = OperatorCode::from_seq(groups[index]);
            if util::op_used(op) {
                let entry = format!("{:02x} {:12} {} {}", op.0, op, f(nanos), f(nanos * gas));
                let entry = format!("{entry:29}");
                print!("{}", entry.color(util::op_color(op)));

                col += 1;
                if col % 4 == 0 {
                    println!();
                }
            }
        }
        println!();
        let grey = |name: &str, index: usize| {
            let nanos = self.weights[OP_COUNT + index];
            let entry = format!("-- {name} {} {}", f(nanos), f(nanos * gas));
            print!("{}", format!("{entry:29}").grey());
        };
        grey("Fixed", 0);
        grey("Grace", 1);
        grey("Traps", 2);
        println!();
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
    fn new(size: usize, groups: &Groups) -> Pop {
        let mut models = vec![];
        for _ in 0..size {
            models.push(Model::random(groups));
        }
        Pop {
            best: models[0].clone(),
            models,
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

    fn mix(&self) -> Model {
        let mut mixed = Model::default();
        let count = self.models.len();
        for model in self.models.iter().take(count) {
            for (i, weight) in model.weights.iter().enumerate() {
                mixed.weights[i] += weight / count as f64;
            }
        }
        mixed
    }
}

struct Feed<const N: usize> {
    trials: [VecDeque<Trial>; N],
}

impl<const N: usize> Feed<N> {
    fn new(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(path)?);
        let mut all_trials = VecDeque::new();
        for line in file.lines() {
            all_trials.push_back(line?.parse()?);
        }

        let mut trials = [0; N].map(|_| VecDeque::new());
        trials[0] = all_trials;
        Ok(Self { trials })
    }

    fn batch(&mut self, size: usize) -> Result<Vec<Trial>> {
        let mut trials = vec![];
        let mut bucket = N - 1;
        while trials.len() < size {
            bucket = (bucket + 1) % N;
            let Some(trial) = self.trials[bucket].pop_front() else {
                continue;
            };
            trials.push(trial);
        }
        Ok(trials)
    }

    fn insert(&mut self, mut trial: Trial, promote: bool) {
        let bucket = &mut trial.bucket;
        if promote {
            *bucket = bucket.saturating_sub(1);
        } else {
            *bucket = (*bucket + 1).min(N - 1);
        }
        self.trials[*bucket].push_back(trial);
    }
}

pub fn default_groups() -> Groups {
    let mut groups = [0; OP_COUNT];
    for i in 0..OP_COUNT {
        groups[i] = i;
    }
    groups
}

pub fn groups() -> Groups {
    let mut groups = [0; OP_COUNT];
    for i in 0..OP_COUNT {
        groups[i] = i;
    }
    macro_rules! set_range {
        ($range:expr, $i:expr) => {
            //assert!($range.contains(&$i));
            assert!($range.start() >= &$i);
            for code in $range {
                groups[OperatorCode(code).seq()] = OperatorCode($i).seq();
            }
        };
    }

    set_range!(0x01..=0x02, 0x01); // NOPs
    set_range!(0x41..=0x42, 0x01); // NOPs

    set_range!(0x0c..=0x0d, 0x0c); // branching

    set_range!(0x46..=0x4f, 0x46); // i32 comparisons
    set_range!(0x50..=0x5a, 0x50); // i64 comparisons
    set_range!(0x67..=0x69, 0x67); // i32 bit counters
    set_range!(0x79..=0x7b, 0x79); // i64 bit counters

    set_range!(0x6a..=0x6b, 0x6a); // fast i32 bin ops
    set_range!(0x71..=0x78, 0x6a); // fast i32 bin ops
    set_range!(0x7c..=0x7d, 0x7c); // fast i64 bin ops
    set_range!(0x83..=0x8a, 0x7c); // fast i64 bin ops

    set_range!(0xc0..=0xc1, 0xc0); // i32 extensions
    set_range!(0xc2..=0xc4, 0xac); // i64 extensions

    set_range!(0x6d..=0x70, 0x6d); // i32 divisions
    set_range!(0x7f..=0x82, 0x7f); // i64 divisions

    groups
}

pub fn model(path: &Path, output: &Path) -> Result<()> {
    let mut feed: Feed<2> = Feed::new(path)?;
    let mut save = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(output)?;

    let groups = groups();
    let mut pop = Pop::new(256, &groups);

    for gen in 0.. {
        let trials = feed.batch(512)?;
        let trial_time = trials.iter().map(|x| x.nanos).saturating_sum();
        let trial_time = trial_time / trials.len();

        for model in &mut pop.models {
            let mut total = 0.;
            for trial in &trials {
                total += model.eval(trial, &groups);
            }
            model.fitness = Some(total as usize / trials.len());
        }

        let curr_best = pop.best.fitness.unwrap_or(usize::MAX);

        pop.sort();

        for trial in trials {
            let (error, _) = pop.models[0].error(&trial, &groups);
            feed.insert(trial, error < 0.);
        }

        let (pop_fitness, pop_best) = pop.stats();
        let percent = 100. * pop_best as f64 / trial_time as f64;
        println!(
            "Gen {gen} {percent:.2}% {} {} {}",
            util::format_nanos(curr_best),
            util::format_nanos(pop_best),
            util::format_nanos(pop_fitness)
        );

        let done = feed.trials[0].is_empty();
        if pop_best < curr_best {
            pop.best = pop.models[0].clone();
        }
        if pop_best < curr_best || gen % 100 == 0 || done {
            writeln!(&mut save, "{}", pop.models[0].print_data())?;
            save.flush()?;
        }
        if (gen != 0 && gen % 500 == 0) || done {
            pop.models[0].full_eval(path, &groups)?;
            for (i, trials) in feed.trials.iter().enumerate() {
                println!("{i}: {}", trials.len());
            }
            pop.mix().full_eval(path, &groups)?;
        }
        if done {
            return Ok(())
        }

        let mut new_pop = vec![];
        while new_pop.len() < pop.len() {
            let a = pop.select();
            let b = pop.select();

            let mut child = a.cross_over(b);
            child.mutate(&groups);
            new_pop.push(child);
        }

        pop.models = new_pop;
    }

    unreachable!()
}
