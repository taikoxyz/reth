//! Taiko's payload builder module.

use reth_basic_payload_builder::{
    BuildArguments, BuildOutcome, MissingPayloadBehaviour, PayloadBuilder, PayloadConfig,
};
use reth_chainspec::ChainSpec;
use reth_evm::ConfigureEvm;
use reth_payload_primitives::PayloadBuilderError;
use reth_provider::StateProviderFactory;
use reth_transaction_pool::TransactionPool;
use std::sync::Arc;
use taiko_reth_engine_primitives::{TaikoBuiltPayload, TaikoPayloadBuilderAttributes};
use taiko_reth_evm::{TaikoEvmConfig, TaikoExecutorProvider};
use taiko_reth_provider::l1_origin::L1OriginWriter;

/// Taiko's payload builder
#[derive(Debug, Clone)]
pub struct TaikoPayloadBuilder<EvmConfig = TaikoEvmConfig> {
    /// The type responsible for creating the evm.
    block_executor: TaikoExecutorProvider<EvmConfig>,
}

impl<EvmConfig: Clone> TaikoPayloadBuilder<EvmConfig> {
    /// `OptimismPayloadBuilder` constructor.
    pub const fn new(evm_config: EvmConfig, chain_spec: Arc<ChainSpec>) -> Self {
        let block_executor = TaikoExecutorProvider::new(chain_spec, evm_config);
        Self { block_executor }
    }
}

/// Implementation of the [`PayloadBuilder`] trait for [`TaikoPayloadBuilder`].
impl<Pool, Client, EvmConfig> PayloadBuilder<Pool, Client> for TaikoPayloadBuilder<EvmConfig>
where
    Client: StateProviderFactory, /*+ L1OriginWriter*/
    Pool: TransactionPool,
    EvmConfig: ConfigureEvm,
{
    type Attributes = TaikoPayloadBuilderAttributes;
    type BuiltPayload = TaikoBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, Self::Attributes, Self::BuiltPayload>,
    ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
        todo!()
    }

    fn on_missing_payload(
        &self,
        _args: BuildArguments<Pool, Client, Self::Attributes, Self::BuiltPayload>,
    ) -> MissingPayloadBehaviour<Self::BuiltPayload> {
        todo!()
    }

    fn build_empty_payload(
        &self,
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<Self::BuiltPayload, PayloadBuilderError> {
        todo!()
    }
}
