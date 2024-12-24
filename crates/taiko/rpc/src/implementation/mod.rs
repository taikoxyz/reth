//! A [Consensus] implementation for local testing purposes
//! that automatically seals blocks.
//!
//! The Mining task polls a [`MiningMode`], and will return a list of transactions that are ready to
//! be mined.
//!
//! These downloaders poll the miner, assemble the block, and return transactions that are ready to
//! be mined.

mod client;
mod proposer;
mod task;

use crate::ProvingPreflight;
use alloy_eips::BlockId;
use alloy_primitives::Address;
pub use client::TaikoImplClient;
use reth_consensus::noop::NoopConsensus;
use reth_errors::RethError;
use reth_execution_types::TaskResult;
use reth_taiko_chainspec::TaikoChainSpec;
use reth_transaction_pool::TransactionPool;
use std::sync::Arc;
pub use task::TaikoImplTask;
use tokio::sync::oneshot;

/// Builder type for configuring the setup
#[derive(Debug)]
pub struct TaikoImplBuilder<Provider, Pool, BlockExecutor> {
    provider: Provider,
    consensus: NoopConsensus,
    chain_spec: Arc<TaikoChainSpec>,
    pool: Pool,
    block_executor: BlockExecutor,
}

impl<Provider, Pool, BlockExecutor> TaikoImplBuilder<Provider, Pool, BlockExecutor>
where
    Pool: TransactionPool,
{
    /// Creates a new builder instance to configure all parts.
    pub fn new(
        chain_spec: Arc<TaikoChainSpec>,
        provider: Provider,
        pool: Pool,
        block_executor: BlockExecutor,
    ) -> Self {
        Self { provider, consensus: NoopConsensus::default(), pool, block_executor, chain_spec }
    }

    /// Consumes the type and returns all components
    #[track_caller]
    pub fn build(
        self,
    ) -> (NoopConsensus, TaikoImplClient, TaikoImplTask<Provider, Pool, BlockExecutor>) {
        let Self { provider: client, consensus, chain_spec, pool, block_executor: evm_config } =
            self;
        let (trigger_args_tx, trigger_args_rx) = tokio::sync::mpsc::unbounded_channel();
        let auto_client = TaikoImplClient::new(trigger_args_tx);
        let task =
            TaikoImplTask::new(Arc::clone(&chain_spec), client, pool, evm_config, trigger_args_rx);
        (consensus, auto_client, task)
    }
}

/// Message types for the proposer
#[derive(Debug)]
enum TaikoImplMessage {
    PoolContent {
        /// Address of the beneficiary
        beneficiary: Address,
        /// Base fee
        base_fee: u64,
        /// Maximum gas limit for the block
        block_max_gas_limit: u64,
        /// Maximum bytes per transaction list
        max_bytes_per_tx_list: u64,
        /// Local accounts
        local_accounts: Option<Vec<Address>>,
        /// Maximum number of transactions lists
        max_transactions_lists: u64,
        /// Minimum tip
        min_tip: u64,
        /// Response channel
        tx: oneshot::Sender<Result<Vec<TaskResult>, RethError>>,
    },
    ProvingPreFlight {
        block_id: BlockId,
        /// Response channel
        tx: oneshot::Sender<Result<ProvingPreflight, RethError>>,
    },
}
