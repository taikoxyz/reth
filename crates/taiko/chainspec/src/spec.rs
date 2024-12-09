use crate::taiko::{get_taiko_genesis, get_taiko_hardforks, TaikoNamedChain};
use once_cell::sync::Lazy;
use reth_chainspec::{BaseFeeParams, BaseFeeParamsKind, ChainSpec};
use std::sync::Arc;

/// The Taiko internal L2 A spec
pub static TAIKO_INTERNAL_L2_A: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::TaikoInternalL2a.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::TaikoInternalL2a),
        hardforks: get_taiko_hardforks(TaikoNamedChain::TaikoInternalL2a).unwrap(),
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
        hardforks: get_taiko_hardforks(TaikoNamedChain::Katla).unwrap(),
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
        hardforks: get_taiko_hardforks(TaikoNamedChain::Hekla).unwrap(),
        deposit_contract: None,
        ..Default::default()
    }
    .into()
});

/// The Taiko Mainnet spec
pub static TAIKO_MAINNET: Lazy<Arc<ChainSpec>> = Lazy::new(|| {
    ChainSpec {
        chain: TaikoNamedChain::Mainnet.into(),
        genesis: get_taiko_genesis(TaikoNamedChain::Mainnet),
        hardforks: get_taiko_hardforks(TaikoNamedChain::Mainnet).unwrap(),
        ..Default::default()
    }
    .into()
});
