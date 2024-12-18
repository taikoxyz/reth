//! Beacon consensus implementation.

use reth_chainspec::ChainSpec;
use reth_consensus::{Consensus, ConsensusError, PostExecutionInput};
use reth_primitives::revm_primitives::U256;
use reth_primitives::{BlockWithSenders, Header, SealedBlock, SealedHeader};
use std::sync::Arc;
use std::time::SystemTime;

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

impl Consensus for TaikoBeaconConsensus {
    fn validate_header(&self, header: &SealedHeader) -> Result<(), ConsensusError> {
        todo!()
    }

    fn validate_header_against_parent(
        &self,
        header: &SealedHeader,
        parent: &SealedHeader,
    ) -> Result<(), ConsensusError> {
        todo!()
    }

    fn validate_header_with_total_difficulty(
        &self,
        header: &Header,
        total_difficulty: U256,
    ) -> Result<(), ConsensusError> {
        todo!()
    }

    fn validate_block_pre_execution(&self, block: &SealedBlock) -> Result<(), ConsensusError> {
        todo!()
    }

    fn validate_block_post_execution(
        &self,
        block: &BlockWithSenders,
        input: PostExecutionInput<'_>,
    ) -> Result<(), ConsensusError> {
        todo!()
    }
}
