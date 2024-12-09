pub mod validator;
pub use validator::*;

use std::sync::Arc;
use reth_chainspec::ChainSpec;
use taiko_reth_chainspec::TaikoChainSpec;

/// Taiko beacon consensus
///
/// This consensus engine does basic checks as outlined in the execution specs.
#[derive(Debug)]
pub struct TaikoBeaconConsensus {
    /// Configuration
    chain_spec: Arc<TaikoChainSpec>,
}

impl TaikoBeaconConsensus {
    /// Create a new instance of [`EthBeaconConsensus`]
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { chain_spec }
    }
}