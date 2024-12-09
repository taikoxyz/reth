use crate::TaikoEvmConfig;
use alloc::sync::Arc;
use core::fmt::Display;
use reth_chainspec::ChainSpec;
use reth_evm::execute::{
    BatchExecutor, BlockExecutionError, BlockExecutionInput, BlockExecutionOutput,
    BlockExecutorProvider, ExecutionOutcome, Executor, ProviderError,
};
use reth_evm::system_calls::OnStateHook;
use reth_evm::ConfigureEvm;
use reth_primitives::revm_primitives::alloy_primitives::BlockNumber;
use reth_primitives::revm_primitives::db::Database;
use reth_primitives::{BlockWithSenders, Header, Receipt};
use reth_prune_types::PruneModes;
use reth_revm::State;

/// Helper container type for EVM with chain spec.
#[derive(Debug, Clone)]
pub struct TaikoEvmExecutor<EvmConfig> {
    /// The chainspec
    chain_spec: Arc<ChainSpec>,
    /// How to create an EVM.
    evm_config: EvmConfig,
}

/// A basic Taiko block executor.
///
/// Expected usage:
/// - Create a new instance of the executor.
/// - Execute the block.
#[derive(Debug)]
pub struct TaikoBlockExecutor<EvmConfig, DB> {
    /// Chain specific evm config that's used to execute a block.
    executor: TaikoEvmExecutor<EvmConfig>,
    /// The state to use for execution
    state: State<DB>,
}

impl<EvmConfig, DB> TaikoBlockExecutor<EvmConfig, DB> {
    /// Creates a new Ethereum block executor.
    pub const fn new(chain_spec: Arc<ChainSpec>, evm_config: EvmConfig, state: State<DB>) -> Self {
        Self { executor: TaikoEvmExecutor { chain_spec, evm_config }, state }
    }

    #[inline]
    fn chain_spec(&self) -> &ChainSpec {
        &self.executor.chain_spec
    }

    /// Returns mutable reference to the state that wraps the underlying database.
    #[allow(unused)]
    fn state_mut(&mut self) -> &mut State<DB> {
        &mut self.state
    }
}

impl<EvmConfig, DB> BatchExecutor<DB> for TaikoBlockExecutor<EvmConfig, DB>
where
    EvmConfig: ConfigureEvm<Header = Header>,
    DB: Database<Error: Into<ProviderError> + Display>,
{
    type Input<'a> = BlockExecutionInput<'a, BlockWithSenders>;
    type Output = ExecutionOutcome;
    type Error = BlockExecutionError;

    fn execute_and_verify_one(&mut self, input: Self::Input<'_>) -> Result<(), Self::Error> {
        todo!()
    }

    fn finalize(self) -> Self::Output {
        todo!()
    }

    fn set_tip(&mut self, tip: BlockNumber) {
        todo!()
    }

    fn set_prune_modes(&mut self, prune_modes: PruneModes) {
        todo!()
    }

    fn size_hint(&self) -> Option<usize> {
        todo!()
    }
}

impl<EvmConfig, DB> Executor<DB> for TaikoBlockExecutor<EvmConfig, DB>
where
    EvmConfig: ConfigureEvm<Header = Header>,
    DB: Database<Error: Into<ProviderError> + Display>,
{
    type Input<'a> = BlockExecutionInput<'a, BlockWithSenders>;
    type Output = BlockExecutionOutput<Receipt>;
    type Error = BlockExecutionError;

    fn execute(self, input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        todo!()
    }

    fn execute_with_state_closure<F>(
        self,
        input: Self::Input<'_>,
        state: F,
    ) -> Result<Self::Output, Self::Error>
    where
        F: FnMut(&State<DB>),
    {
        todo!()
    }

    fn execute_with_state_hook<F>(
        self,
        input: Self::Input<'_>,
        state_hook: F,
    ) -> Result<Self::Output, Self::Error>
    where
        F: OnStateHook,
    {
        todo!()
    }
}

/// Provides executors to execute regular ethereum blocks
#[derive(Debug, Clone)]
pub struct TaikoExecutorProvider<EvmConfig = TaikoEvmConfig> {
    chain_spec: Arc<ChainSpec>,
    evm_config: EvmConfig,
}

impl<EvmConfig> TaikoExecutorProvider<EvmConfig> {
    /// Creates a new executor provider.
    pub const fn new(chain_spec: Arc<ChainSpec>, evm_config: EvmConfig) -> Self {
        Self { chain_spec, evm_config }
    }
}

impl<EvmConfig> BlockExecutorProvider for TaikoExecutorProvider<EvmConfig>
where
    EvmConfig: ConfigureEvm<Header = Header>,
{
    type Executor<DB: Database<Error: Into<ProviderError> + Display>> =
        TaikoBlockExecutor<EvmConfig, DB>;
    type BatchExecutor<DB: Database<Error: Into<ProviderError> + Display>> =
        TaikoBlockExecutor<EvmConfig, DB>;

    fn executor<DB>(&self, db: DB) -> Self::Executor<DB>
    where
        DB: Database<Error: Into<ProviderError> + Display>,
    {
        todo!()
    }

    fn batch_executor<DB>(&self, db: DB) -> Self::BatchExecutor<DB>
    where
        DB: Database<Error: Into<ProviderError> + Display>,
    {
        todo!()
    }
}
