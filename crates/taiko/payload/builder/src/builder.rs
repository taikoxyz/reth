//! Taiko's payload builder module.

use std::sync::Arc;
use reth_chainspec::ChainSpec;
use taiko_reth_evm::{TaikoEvmConfig, TaikoExecutorProvider};

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