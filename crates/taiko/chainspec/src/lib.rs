//! A [Consensus] implementation for local testing purposes
//! that automatically seals blocks.
//!
//! The Mining task polls a [`MiningMode`], and will return a list of transactions that are ready to
//! be mined.
//!
//! These downloaders poll the miner, assemble the block, and return transactions that are ready to
//! be mined.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]

use std::fmt::Display;
use std::sync::Arc;
use derive_more::{Constructor, Deref, Into};
use reth_chainspec::{BaseFeeParams, Chain, ChainSpec, DepositContract, EthChainSpec, EthereumHardforks, ForkCondition, ForkFilter, ForkId, Hardfork, Hardforks, Head};
use reth_primitives::Header;
use reth_primitives::revm_primitives::{B256, U256};

/// OP stack chain spec type.
#[derive(Debug, Clone, Deref, Into, PartialEq, Eq)]
pub struct TaikoChainSpec {
    /// [`ChainSpec`].
    pub inner: Arc<ChainSpec>,
}

impl TaikoChainSpec {
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { inner: chain_spec }
    }
}

impl EthChainSpec for TaikoChainSpec {
    fn chain(&self) -> Chain {
        self.inner.chain()
    }

    fn base_fee_params_at_block(&self, block_number: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_block(block_number)
    }

    fn base_fee_params_at_timestamp(&self, timestamp: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_timestamp(timestamp)
    }

    fn deposit_contract(&self) -> Option<&DepositContract> {
        self.inner.deposit_contract()
    }

    fn genesis_hash(&self) -> B256 {
        self.inner.genesis_hash()
    }

    fn prune_delete_limit(&self) -> usize {
        self.inner.prune_delete_limit()
    }

    fn display_hardforks(&self) -> impl Display {
        self.inner.display_hardforks()
    }

    fn genesis_header(&self) -> &Header {
        self.inner.genesis_header()
    }

    fn genesis(&self) -> &alloy_genesis::Genesis {
        self.inner.genesis()
    }

    fn max_gas_limit(&self) -> u64 {
        self.inner.max_gas_limit()
    }

    fn bootnodes(&self) -> Option<Vec<reth_network_peers::node_record::NodeRecord>> {
        self.inner.bootnodes()
    }
}

impl Hardforks for TaikoChainSpec {
    fn fork<H: Hardfork>(&self, fork: H) -> ForkCondition {
        self.inner.fork(fork)
    }

    fn forks_iter(&self) -> impl Iterator<Item=(&dyn Hardfork, ForkCondition)> {
        self.inner.forks_iter()
    }

    fn fork_id(&self, head: &Head) -> ForkId {
        self.inner.fork_id(head)
    }

    fn latest_fork_id(&self) -> ForkId {
        self.inner.latest_fork_id()
    }

    fn fork_filter(&self, head: Head) -> ForkFilter {
        self.inner.fork_filter(head)
    }
}

impl EthereumHardforks for TaikoChainSpec {
    fn get_final_paris_total_difficulty(&self) -> Option<U256> {
        self.inner.get_final_paris_total_difficulty()
    }

    fn final_paris_total_difficulty(&self, block_number: u64) -> Option<U256> {
        self.inner.final_paris_total_difficulty(block_number)
    }
}