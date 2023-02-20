use prover::programs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// Defines configuration options for Stylus projects
/// via TOML or CLI flags. This affects the Stylus'
/// instrumentation pipeline.
pub struct ProjectConfig {
    pub resources: Resources,
    pub pricing: Pricing,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            resources: Resources::default(),
            pricing: Pricing::default(),
        }
    }
}

/// Defines the gas resources alloted to a Stylus program
#[derive(Serialize, Deserialize)]
pub struct Resources {
    pub start_gas: u64,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            start_gas: 1_000_000,
        }
    }
}

/// Defines the pricing details for instrumenting a Stylus program
#[derive(Serialize, Deserialize)]
pub struct Pricing {
    pub wasm_gas_price: u64,
    pub hostio_cost: u64,
}

impl Default for Pricing {
    fn default() -> Self {
        Self {
            wasm_gas_price: 100_00,
            hostio_cost: 100,
        }
    }
}

impl From<ProjectConfig> for StylusConfig {
    fn from(pcfg: ProjectConfig) -> Self {
        let mut config = StylusConfig::default();
        config.add_debug_params();
        config.start_gas = pcfg.resources.start_gas;
        config.pricing.wasm_gas_price = pcfg.pricing.wasm_gas_price;
        config.pricing.hostio_cost = pcfg.pricing.hostio_cost;
        // TODO: Further customize.
        config.costs = |_| 1;
        config
    }
}
