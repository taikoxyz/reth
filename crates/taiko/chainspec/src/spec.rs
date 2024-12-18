use crate::taiko::{get_taiko_genesis, TaikoNamedChain};
use once_cell::sync::Lazy;
use reth_chainspec::{BaseFeeParams, BaseFeeParamsKind, ChainSpec};
use std::sync::Arc;
use taiko_reth_forks::TaikoHardFork;

/// The Taiko internal L2 A spec
pub static TAIKO_INTERNAL_L2_A: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::TaikoInternalL2a.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::TaikoInternalL2a),
        hardforks: TaikoHardFork::taiko_internal_l2a(),
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams {
            max_change_denominator: 8,
            elasticity_multiplier: 2,
        }),
        ..Default::default()
    }
    .into()
});

/// The Taiko testnet spec
pub static TAIKO_TESTNET: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::Katla.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::Katla),
        hardforks: TaikoHardFork::taiko_testnet(),
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams {
            max_change_denominator: 8,
            elasticity_multiplier: 2,
        }),
        ..Default::default()
    }
    .into()
});

/// The Taiko A7 spec
pub static TAIKO_HEKLA: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::Hekla.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::Hekla),
        hardforks: TaikoHardFork::taiko_hekla(),
        ..Default::default()
    }
    .into()
});

/// The Taiko Mainnet spec
pub static TAIKO_MAINNET: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::Mainnet.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::Mainnet),
        hardforks: TaikoHardFork::taiko_mainnet(),
        ..Default::default()
    }
    .into()
});
