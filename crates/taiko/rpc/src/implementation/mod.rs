//! A [Consensus] implementation for local testing purposes
//! that automatically seals blocks.
//!
//! The Mining task polls a [`MiningMode`], and will return a list of transactions that are ready to
//! be mined.
//!
//! These downloaders poll the miner, assemble the block, and return transactions that are ready to
//! be mined.

mod client;
mod preflight;
mod proposer;
mod task;

use crate::ProvingPreflight;
use alloy_eips::BlockId;
use alloy_primitives::Address;
pub use client::TaikoImplClient;
use reth_consensus::noop::NoopConsensus;
use reth_errors::RethError;
use reth_evm::execute::BlockExecutorProvider;
use reth_execution_types::TaskResult;
use reth_node_api::NodePrimitives;
use reth_primitives::{Block, TransactionSigned};
use reth_provider::{BlockReaderIdExt, StateProviderFactory};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_tasks::TaskSpawner;
use reth_transaction_pool::{PoolTransaction, TransactionPool};
use std::sync::Arc;
pub use task::TaikoImplTask;
use tokio::sync::oneshot;

/// Builder type for configuring the setup
#[derive(Debug)]
pub struct TaikoImplBuilder<Provider, Pool, Executor> {
    provider: Provider,
    consensus: NoopConsensus,
    chain_spec: Arc<TaikoChainSpec>,
    pool: Pool,
    block_executor: Executor,
    task_spawner: Box<dyn TaskSpawner>,
}

impl<Provider, Pool, Executor> TaikoImplBuilder<Provider, Pool, Executor>
where
    Provider: StateProviderFactory
        + BlockReaderIdExt<Header = reth_primitives::Header>
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
    Executor: BlockExecutorProvider,
    Executor::Primitives: NodePrimitives<Block = Block>,
{
    /// Creates a new builder instance to configure all parts.
    pub fn new(
        chain_spec: Arc<TaikoChainSpec>,
        provider: Provider,
        pool: Pool,
        block_executor: Executor,
        task_spawner: Box<dyn TaskSpawner>,
    ) -> Self {
        Self {
            provider,
            consensus: NoopConsensus::default(),
            pool,
            block_executor,
            chain_spec,
            task_spawner,
        }
    }

    /// Consumes the type and returns all components
    #[track_caller]
    pub fn build(self) -> (NoopConsensus, TaikoImplClient) {
        let Self { provider, consensus, chain_spec, pool, block_executor, task_spawner } = self;
        let (pool_content_tx, pool_content_rx) = tokio::sync::mpsc::unbounded_channel();
        let (proving_preflight_tx, proving_preflight_rx) = tokio::sync::mpsc::unbounded_channel();
        let auto_client = TaikoImplClient::new(pool_content_tx, proving_preflight_tx);
        let task = TaikoImplTask::new(
            task_spawner.clone(),
            Arc::clone(&chain_spec),
            provider.clone(),
            pool.clone(),
            block_executor.clone(),
            pool_content_rx,
        );
        task_spawner.spawn(Box::pin(task));
        let task = TaikoImplTask::new(
            task_spawner.clone(),
            Arc::clone(&chain_spec),
            provider,
            pool,
            block_executor,
            proving_preflight_rx,
        );
        task_spawner.spawn(Box::pin(task));
        (consensus, auto_client)
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
