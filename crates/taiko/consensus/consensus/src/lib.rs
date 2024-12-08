pub mod validator;
pub use validator::*;

use std::sync::Arc;
use reth_chainspec::ChainSpec;

/// Taiko beacon consensus
///
/// This consensus engine does basic checks as outlined in the execution specs.
#[derive(Debug)]
pub struct TaikoBeaconConsensus {
    /// Configuration
    chain_spec: Arc<ChainSpec>,
}

impl TaikoBeaconConsensus {
    /// Create a new instance of [`EthBeaconConsensus`]
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }
}