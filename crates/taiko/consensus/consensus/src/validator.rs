use std::sync::Arc;
use taiko_reth_chainspec::TaikoChainSpec;

/// Validator for Optimism engine API.
#[derive(Debug, Clone)]
pub struct TaikoEngineValidator {
    chain_spec: Arc<TaikoChainSpec>,
}

impl TaikoEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { chain_spec }
    }
}