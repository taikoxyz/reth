use alloy_eips::BlockId;
use reth_errors::RethError;
use reth_evm::execute::BlockExecutorProvider;
use reth_node_api::NodePrimitives;
use reth_primitives::Block;
use reth_provider::{BlockReaderIdExt, StateProviderFactory};

use crate::ProvingPreflight;

/// Builds and executes a new block with the given transactions, on the provided executor.
///
/// This returns the header of the executed block, as well as the poststate from execution.
#[allow(clippy::too_many_arguments)]
pub fn build_and_execute<Provider, Executor>(
    provider: &Provider,
    executor: &Executor,
    block_id: BlockId,
) -> Result<ProvingPreflight, RethError>
where
    Executor: BlockExecutorProvider,
    Executor::Primitives: NodePrimitives<Block = Block>,
    Provider: StateProviderFactory + BlockReaderIdExt<Header = reth_primitives::Header>,
{
    todo!()
}
