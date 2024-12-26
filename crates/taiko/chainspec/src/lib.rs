//! OP-Reth chain specs.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use alloy_chains::Chain;
use alloy_consensus::Header;
use alloy_genesis::{ChainConfig, Genesis, GenesisAccount};
use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
use core::str::FromStr;
use derive_more::{Constructor, Deref, Display, From, Into};
#[cfg(not(feature = "std"))]
pub(crate) use once_cell::sync::Lazy as LazyLock;
use reth_chainspec::{
    BaseFeeParams, ChainSpec, ChainSpecBuilder, DepositContract, EthChainSpec, EthereumHardforks,
    ForkFilter, ForkId, Hardforks, Head,
};
use reth_ethereum_forks::{ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};
use reth_network_peers::NodeRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "std")]
pub(crate) use std::sync::LazyLock;

use reth_taiko_forks::{
    TaikoHardfork, TaikoHardforks, CHAIN_HEKLA_TESTNET, CHAIN_INTERNAL_TESTNET,
    CHAIN_KATLA_TESTNET, CHAIN_MAINNET, CHAIN_PERCONF_DEVNET,
};

// Taiko Chain Configuration, sets the chain_id to the internal devnet L2A by default.
static TAIKO_CHAIN_CONFIG: LazyLock<ChainConfig> = LazyLock::new(|| ChainConfig {
    chain_id: 0,
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
    osaka_time: None,
    parlia: None,
    deposit_contract_address: None,
});

/// An account in the state of the genesis block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaikoGenesisAccount {
    /// The nonce of the account at genesis.
    #[serde(skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt", default)]
    pub nonce: Option<u64>,
    /// The balance of the account at genesis.
    pub balance: U256,
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

/// Returns the genesis block for the given chain.
pub fn get_taiko_genesis(chain: Chain) -> Genesis {
    let alloc_str = match chain {
        CHAIN_MAINNET => {
            include_str!("../res/genesis/mainnet.json")
        }
        CHAIN_INTERNAL_TESTNET => {
            include_str!("../res/genesis/internal_l2a.json")
        }
        CHAIN_KATLA_TESTNET => include_str!("../res/genesis/katla.json"),
        CHAIN_HEKLA_TESTNET => include_str!("../res/genesis/hekla.json"),
        CHAIN_PERCONF_DEVNET => include_str!("../res/genesis/preconf_devnet.json"),
        _ => panic!("Invalid chain"),
    };

    let alloc: BTreeMap<Address, TaikoGenesisAccount> =
        serde_json::from_str(alloc_str).expect("Invalid alloc json");
    let mut config = TAIKO_CHAIN_CONFIG.clone();
    config.chain_id = chain.id();

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

/// Chain spec builder for a OP stack chain.
#[derive(Debug, Default, From)]
pub struct TaikoChainSpecBuilder {
    /// [`ChainSpecBuilder`]
    inner: ChainSpecBuilder,
}

impl TaikoChainSpecBuilder {
    /// Set the chain ID
    pub fn chain(mut self, chain: Chain) -> Self {
        self.inner = self.inner.chain(chain);
        self
    }

    /// Set the genesis block.
    pub fn genesis(mut self, genesis: Genesis) -> Self {
        self.inner = self.inner.genesis(genesis);
        self
    }

    /// Add the given fork with the given activation condition to the spec.
    pub fn with_fork<H: Hardfork>(mut self, fork: H, condition: ForkCondition) -> Self {
        self.inner = self.inner.with_fork(fork, condition);
        self
    }

    /// Add the given forks with the given activation condition to the spec.
    pub fn with_forks(mut self, forks: ChainHardforks) -> Self {
        self.inner = self.inner.with_forks(forks);
        self
    }

    /// Remove the given fork from the spec.
    pub fn without_fork(mut self, fork: reth_taiko_forks::TaikoHardfork) -> Self {
        self.inner = self.inner.without_fork(fork);
        self
    }

    /// Enable Regolith at genesis
    pub fn ontake_activated(mut self) -> Self {
        self.inner =
            self.inner.with_fork(reth_taiko_forks::TaikoHardfork::Ontake, ForkCondition::Block(0));
        self
    }

    /// Build the resulting [`TaikoChainSpec`].
    ///
    /// # Panics
    ///
    /// This function panics if the chain ID and genesis is not set ([`Self::chain`] and
    /// [`Self::genesis`])
    pub fn build(self) -> TaikoChainSpec {
        TaikoChainSpec { inner: self.inner.build() }
    }
}

/// OP stack chain spec type.
#[derive(Debug, Clone, Deref, Into, Constructor, PartialEq, Eq)]
pub struct TaikoChainSpec {
    /// [`ChainSpec`].
    pub inner: ChainSpec,
}

impl TaikoChainSpec {
    /// Get treasury address.
    pub fn treasury(&self) -> Address {
        const SUFFIX: &str = "10001";
        let prefix = self.chain().id().to_string();
        Address::from_str(&format!(
            "{prefix}{}{SUFFIX}",
            "0".repeat(Address::len_bytes() * 2 - prefix.len() - SUFFIX.len())
        ))
        .unwrap()
    }
}

impl EthChainSpec for TaikoChainSpec {
    type Header = Header;

    fn chain(&self) -> alloy_chains::Chain {
        self.inner.chain()
    }

    fn base_fee_params_at_block(&self, block_number: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_block(block_number)
    }

    fn base_fee_params_at_timestamp(&self, timestamp: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_timestamp(timestamp)
    }

    fn deposit_contract(&self) -> Option<&DepositContract> {
        self.inner.deposit_contract()
    }

    fn genesis_hash(&self) -> B256 {
        self.inner.genesis_hash()
    }

    fn prune_delete_limit(&self) -> usize {
        self.inner.prune_delete_limit()
    }

    fn display_hardforks(&self) -> Box<dyn Display> {
        Box::new(ChainSpec::display_hardforks(self))
    }

    fn genesis_header(&self) -> &Self::Header {
        self.inner.genesis_header()
    }

    fn genesis(&self) -> &Genesis {
        self.inner.genesis()
    }

    fn max_gas_limit(&self) -> u64 {
        self.inner.max_gas_limit()
    }

    fn bootnodes(&self) -> Option<Vec<NodeRecord>> {
        self.inner.bootnodes()
    }

    fn is_taiko(&self) -> bool {
        true
    }

    fn is_optimism(&self) -> bool {
        false
    }
}

impl Hardforks for TaikoChainSpec {
    fn fork<H: reth_chainspec::Hardfork>(&self, fork: H) -> reth_chainspec::ForkCondition {
        self.inner.fork(fork)
    }

    fn forks_iter(
        &self,
    ) -> impl Iterator<Item = (&dyn reth_chainspec::Hardfork, reth_chainspec::ForkCondition)> {
        self.inner.forks_iter()
    }

    fn fork_id(&self, head: &Head) -> ForkId {
        self.inner.fork_id(head)
    }

    fn latest_fork_id(&self) -> ForkId {
        self.inner.latest_fork_id()
    }

    fn fork_filter(&self, head: Head) -> ForkFilter {
        self.inner.fork_filter(head)
    }
}

impl EthereumHardforks for TaikoChainSpec {
    fn get_final_paris_total_difficulty(&self) -> Option<U256> {
        self.inner.get_final_paris_total_difficulty()
    }

    fn final_paris_total_difficulty(&self, block_number: u64) -> Option<U256> {
        self.inner.final_paris_total_difficulty(block_number)
    }
}

impl TaikoHardforks for TaikoChainSpec {}

/// The internal devnet ontake height.
const INTERNAL_DEVNET_ONTAKE_BLOCK: u64 = 0;
/// The hekla ontake height.
const HEKLA_ONTAKE_BLOCK: u64 = 840_512;
/// The mainnet ontake height.
const MAINNET_ONTAKE_BLOCK: u64 = 538_304;

impl From<Genesis> for TaikoChainSpec {
    fn from(genesis: Genesis) -> Self {
        let ontake_block = match Chain::from_id(genesis.config.chain_id) {
            CHAIN_INTERNAL_TESTNET => Some(INTERNAL_DEVNET_ONTAKE_BLOCK),
            CHAIN_HEKLA_TESTNET => Some(HEKLA_ONTAKE_BLOCK),
            CHAIN_MAINNET => Some(MAINNET_ONTAKE_BLOCK),
            _ => None,
        };

        // Block-based hardforks
        let hardfork_opts = [
            (EthereumHardfork::Homestead.boxed(), genesis.config.homestead_block),
            (EthereumHardfork::Tangerine.boxed(), genesis.config.eip150_block),
            (EthereumHardfork::SpuriousDragon.boxed(), genesis.config.eip155_block),
            (EthereumHardfork::Byzantium.boxed(), genesis.config.byzantium_block),
            (EthereumHardfork::Constantinople.boxed(), genesis.config.constantinople_block),
            (EthereumHardfork::Petersburg.boxed(), genesis.config.petersburg_block),
            (EthereumHardfork::Istanbul.boxed(), genesis.config.istanbul_block),
            (EthereumHardfork::MuirGlacier.boxed(), genesis.config.muir_glacier_block),
            (EthereumHardfork::Berlin.boxed(), genesis.config.berlin_block),
            (EthereumHardfork::London.boxed(), genesis.config.london_block),
            (EthereumHardfork::ArrowGlacier.boxed(), genesis.config.arrow_glacier_block),
            (EthereumHardfork::GrayGlacier.boxed(), genesis.config.gray_glacier_block),
            (TaikoHardfork::Ontake.boxed(), ontake_block),
        ];
        let mut block_hardforks = hardfork_opts
            .into_iter()
            .filter_map(|(hardfork, opt)| opt.map(|block| (hardfork, ForkCondition::Block(block))))
            .collect::<Vec<_>>();

        // Paris
        let paris_block_and_final_difficulty =
            if let Some(ttd) = genesis.config.terminal_total_difficulty {
                block_hardforks.push((
                    EthereumHardfork::Paris.boxed(),
                    ForkCondition::TTD {
                        total_difficulty: ttd,
                        fork_block: genesis.config.merge_netsplit_block,
                    },
                ));

                genesis.config.merge_netsplit_block.map(|block| (block, ttd))
            } else {
                None
            };

        // Time-based hardforks
        let time_hardfork_opts = [
            (EthereumHardfork::Shanghai.boxed(), genesis.config.shanghai_time),
            (EthereumHardfork::Cancun.boxed(), genesis.config.cancun_time),
            (EthereumHardfork::Prague.boxed(), genesis.config.prague_time),
        ];

        let mut time_hardforks = time_hardfork_opts
            .into_iter()
            .filter_map(|(hardfork, opt)| {
                opt.map(|time| (hardfork, ForkCondition::Timestamp(time)))
            })
            .collect::<Vec<_>>();

        block_hardforks.append(&mut time_hardforks);

        Self {
            inner: ChainSpec {
                chain: genesis.config.chain_id.into(),
                genesis,
                hardforks: ChainHardforks::new(block_hardforks),
                paris_block_and_final_difficulty,
                ..Default::default()
            },
        }
    }
}
