//! Taiko Chain Specification
use std::collections::BTreeMap;

use alloy_chains::Chain;
use alloy_genesis::{ChainConfig, Genesis, GenesisAccount};
use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
use core::{fmt, fmt::Formatter, str::FromStr};
use once_cell::sync::Lazy;
use reth_ethereum_forks::{hardfork, ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};
use serde::{Deserialize, Serialize};
use taiko_reth_forks::TaikoHardFork;

/// The internal devnet ontake height.
pub const INTERNAL_DEVNET_ONTAKE_BLOCK: u64 = 2;
/// The hekla ontake height.
pub const HEKLA_ONTAKE_BLOCK: u64 = 840_512;
/// The mainnet ontake height.
pub const MAINNET_ONTAKE_BLOCK: u64 = 9_000_000;

// Taiko Chain Configuration, sets the chain_id to the internal devnet L2A by default.
static TAIKO_CHAIN_CONFIG: Lazy<ChainConfig> = Lazy::new(|| ChainConfig {
    chain_id: TaikoNamedChain::TaikoInternalL2a as u64,
    homestead_block: Some(0),
    dao_fork_block: None,
    dao_fork_support: false,
    eip150_block: Some(0),
    eip155_block: Some(0),
    eip158_block: Some(0),
    byzantium_block: Some(0),
    constantinople_block: Some(0),
    petersburg_block: Some(0),
    istanbul_block: Some(0),
    muir_glacier_block: None,
    berlin_block: Some(0),
    london_block: Some(0),
    arrow_glacier_block: None,
    gray_glacier_block: None,
    merge_netsplit_block: None,
    shanghai_time: Some(0),
    cancun_time: None,
    terminal_total_difficulty: Some(U256::ZERO),
    terminal_total_difficulty_passed: true,
    ethash: None,
    clique: None,
    extra_fields: Default::default(),
    prague_time: None,
    parlia: None,
    deposit_contract_address: None,
});

/// The named chains for Taiko.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::IntoStaticStr)] // NamedChain::VARIANTS
#[derive(strum::VariantNames)] // NamedChain::VARIANTS
#[derive(strum::VariantArray)] // NamedChain::VARIANTS
#[derive(strum::EnumString)] // FromStr, TryFrom<&str>
#[derive(strum::EnumIter)] // NamedChain::iter
#[derive(strum::EnumCount)] // NamedChain::COUNT
#[derive(num_enum::TryFromPrimitive)] // TryFrom<u64>
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[strum(serialize_all = "kebab-case")]
#[repr(u64)]
#[non_exhaustive]
pub enum TaikoNamedChain {
    /// The mainnet chain.
    #[cfg_attr(feature = "serde", serde(alias = "mainnet"))]
    Mainnet = 167000,
    /// The internal devnet L2A chain.
    #[cfg_attr(feature = "serde", serde(alias = "taiko-internal-l2a"))]
    TaikoInternalL2a = 167001,
    /// The internal devnet L2B chain.
    #[cfg_attr(feature = "serde", serde(alias = "taiko-internal-l2b"))]
    TaikoInternalL2b = 167002,
    /// The Snaefellsjokull chain.
    #[cfg_attr(feature = "serde", serde(alias = "snaefellsjokull"))]
    Snaefellsjokull = 167003,
    /// The Askja chain.
    #[cfg_attr(feature = "serde", serde(alias = "askja"))]
    Askja = 167004,
    /// The Grimsvotn chain.
    #[cfg_attr(feature = "serde", serde(alias = "grimsvotn"))]
    Grimsvotn = 167005,
    /// The Eldfell chain.
    #[cfg_attr(feature = "serde", serde(alias = "eldfell"))]
    Eldfell = 167006,
    /// The Jolnir chain.
    #[cfg_attr(feature = "serde", serde(alias = "jolnir"))]
    Jolnir = 167007,
    /// The Katla chain.
    #[cfg_attr(feature = "serde", serde(alias = "katla"))]
    Katla = 167008,
    /// The Hekla chain.
    #[cfg_attr(feature = "serde", serde(alias = "hekla"))]
    Hekla = 167009,
}

impl From<TaikoNamedChain> for Chain {
    fn from(val: TaikoNamedChain) -> Self {
        Self::from_id_unchecked(val as u64)
    }
}

/// Returns the genesis block for the given chain.
pub fn get_taiko_genesis(chain: TaikoNamedChain) -> Genesis {
    let alloc_str = match chain {
        TaikoNamedChain::Mainnet => {
            include_str!("../res/genesis/mainnet.json")
        }
        TaikoNamedChain::TaikoInternalL2a => {
            include_str!("../res/genesis/internal_l2a.json")
        }
        TaikoNamedChain::TaikoInternalL2b => {
            include_str!("../res/genesis/internal_l2b.json")
        }
        TaikoNamedChain::Snaefellsjokull => {
            include_str!("../res/genesis/snaefellsjokull.json")
        }
        TaikoNamedChain::Askja => include_str!("../res/genesis/askja.json"),
        TaikoNamedChain::Grimsvotn => include_str!("../res/genesis/grimsvotn.json"),
        TaikoNamedChain::Eldfell => include_str!("../res/genesis/eldfell.json"),
        TaikoNamedChain::Jolnir => include_str!("../res/genesis/jolnir.json"),
        TaikoNamedChain::Katla => include_str!("../res/genesis/katla.json"),
        TaikoNamedChain::Hekla => include_str!("../res/genesis/hekla.json"),
    };

    let alloc: BTreeMap<Address, TaikoGenesisAccount> =
        serde_json::from_str(alloc_str).expect("Invalid alloc json");
    let mut config = TAIKO_CHAIN_CONFIG.clone();
    config.chain_id = chain as u64;

    Genesis {
        config,
        alloc: alloc.into_iter().map(|(k, v)| (k, v.into())).collect(),
        nonce: 0,
        timestamp: 0,
        extra_data: Bytes::new(),
        gas_limit: 15_000_000,
        difficulty: U256::ZERO,
        mix_hash: FixedBytes::ZERO,
        coinbase: Address::ZERO,
        base_fee_per_gas: Some(10_000_000),
        excess_blob_gas: None,
        blob_gas_used: None,
        number: None,
    }
}

/// An account in the state of the genesis block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaikoGenesisAccount {
    /// The nonce of the account at genesis.
    #[serde(skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt", default)]
    pub nonce: Option<u64>,
    /// The balance of the account at genesis.
    pub balance: alloy_primitives::ruint::aliases::U256,
    /// The account's bytecode at genesis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Bytes>,
    /// The account's storage at genesis.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "alloy_serde::storage::deserialize_storage_map"
    )]
    pub storage: Option<BTreeMap<B256, B256>>,
    /// The account's private key. Should only be used for testing.
    #[serde(rename = "secretKey", default, skip_serializing_if = "Option::is_none")]
    pub private_key: Option<B256>,
}

impl From<TaikoGenesisAccount> for GenesisAccount {
    fn from(account: TaikoGenesisAccount) -> Self {
        Self {
            nonce: account.nonce,
            balance: account.balance,
            code: account.code,
            storage: account.storage,
            private_key: account.private_key,
        }
    }
}
