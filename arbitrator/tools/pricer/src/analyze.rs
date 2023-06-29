// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::model::{groups, Model, Trial};
use eyre::{eyre, Result};
use rev_lines::RevLines;
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

fn last_line(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut lines = RevLines::new(file);
    let line = lines.next().ok_or(eyre!("no line"))??;
    Ok(line)
}

pub fn analyze(model: Option<PathBuf>, trials: Option<PathBuf>) -> Result<()> {
    if let Some(path) = model {
        let model = Model::from_str(&last_line(&path)?)?;
        model.print(&groups());
        if let Some(trials) = &trials {
            model.full_eval(&trials, &groups())?;
        }
    } else {
        if let Some(path) = trials {
            let trial = Trial::from_str(&last_line(&path)?)?;
            trial.print();
        }
    }
    Ok(())
}
