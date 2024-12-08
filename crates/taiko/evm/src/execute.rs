use alloc::sync::Arc;
use reth_chainspec::ChainSpec;
use crate::TaikoEvmConfig;

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