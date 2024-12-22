//! EVM config for vanilla ethereum.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod dao_fork;
pub mod eip6110;
pub mod execute;

use core::str::FromStr;
pub use execute::*;
use reth_chainspec::ChainSpec;
use reth_ethereum_forks::Head;
use reth_evm::{ConfigureEvm, ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_primitives::revm_primitives::db::Database;
use reth_primitives::revm_primitives::{
    Address, AnalysisKind, BlockEnv, Bytes, CfgEnvWithHandlerCfg, Env, TaikoFields, TxEnv, TxKind,
    U256,
};
use reth_primitives::{revm_primitives, transaction::FillTxEnv, Header, TransactionSigned};
use reth_revm::{inspector_handle_register, Evm, EvmBuilder, GetInspector};
use taiko_reth_forks::TaikoHardFork;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::sync::Arc;

/// Ethereum-related EVM configuration.
#[derive(Debug, Clone, Default)]
pub struct TaikoEvmConfig {
    chain_spec: Arc<ChainSpec>,
}

impl TaikoEvmConfig {
    /// Creates a new [`OptimismEvmConfig`] with the given chain spec.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }

    /// Returns the chain spec associated with this configuration.
    pub fn chain_spec(&self) -> &ChainSpec {
        &self.chain_spec
    }
}

impl ConfigureEvmEnv for TaikoEvmConfig {
    type Header = Header;

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        transaction.fill_tx_env(tx_env, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        let chain_spec = self.chain_spec();
        let tx = TxEnv {
            caller,
            transact_to: TxKind::Call(contract),
            // Explicitly set nonce to None so revm does not do any nonce checks
            nonce: None,
            gas_limit: 30_000_000,
            value: U256::ZERO,
            data,
            // Setting the gas price to zero enforces that no value is transferred as part of the
            // call, and that the call will not count against the block's gas limit
            gas_price: U256::ZERO,
            // The chain ID check is not relevant here and is disabled if set to None
            chain_id: None,
            // Setting the gas priority fee to None ensures the effective gas price is derived from
            // the `gas_price` field, which we need to be zero
            gas_priority_fee: None,
            access_list: Vec::new(),
            // blob fields can be None for this tx
            blob_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            authorization_list: None,
            taiko: TaikoFields {
                treasury: treasury(chain_spec.chain().id().to_string()),
                basefee_ratio: 0,
                is_anchor: false,
            },
        };
        env.tx = tx;

        // ensure the block gas limit is >= the tx
        env.block.gas_limit = U256::from(env.tx.gas_limit);

        // disable the base fee check for this call by setting the base fee to zero
        env.block.basefee = U256::ZERO;
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        total_difficulty: U256,
    ) {
        let chain_spec = self.chain_spec();
        let spec_id = revm_spec(
            chain_spec,
            &Head {
                number: header.number,
                timestamp: header.timestamp,
                difficulty: header.difficulty,
                total_difficulty,
                hash: Default::default(),
            },
        );

        cfg_env.chain_id = chain_spec.chain().id();
        cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Analyse;

        cfg_env.handler_cfg.spec_id = spec_id;
        cfg_env.handler_cfg.is_taiko = true;
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> (CfgEnvWithHandlerCfg, BlockEnv) {
        todo!()
    }
}

impl ConfigureEvm for TaikoEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        EvmBuilder::default().with_db(db).taiko().build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .taiko()
            .append_handler_register(inspector_handle_register)
            .build()
    }

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
}

// Map the latest active hardfork at the given block to a revm [`SpecId`](revm_primitives::SpecId).
pub fn revm_spec(chain_spec: &ChainSpec, block: &Head) -> revm_primitives::SpecId {
    if chain_spec.fork(TaikoHardFork::Ontake).active_at_head(&block) {
        return revm_primitives::ONTAKE;
    } else if chain_spec.fork(TaikoHardFork::Hekla).active_at_head(&block) {
        return revm_primitives::HEKLA;
    }
    reth_evm_ethereum::revm_spec(chain_spec, block)
}

/// Returns the treasury address for the chain.
pub fn treasury(chain_id: String) -> Address {
    const SUFFIX: &str = "10001";
    Address::from_str(&format!(
        "{chain_id}{}{SUFFIX}",
        "0".repeat(Address::len_bytes() * 2 - chain_id.len() - SUFFIX.len())
    ))
    .unwrap()
}
