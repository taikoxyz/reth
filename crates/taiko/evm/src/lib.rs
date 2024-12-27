//! EVM config for vanilla optimism.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(feature = "std"), no_std)]
// The `optimism` feature must be enabled to use this crate.

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::Header;
use alloy_primitives::{Address, U256};
use core::convert::Infallible;
use reth_evm::{ConfigureEvm, ConfigureEvmEnv, EnvExt, NextBlockEnvAttributes};
use reth_primitives::{transaction::FillTxEnv, Head, TransactionSigned};
use reth_revm::{
    inspector_handle_register,
    primitives::{AnalysisKind, CfgEnvWithHandlerCfg, TaikoFields, TxEnv},
    Database, Evm, EvmBuilder, GetInspector,
};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_consensus::decode_ontake_extra_data;
use reth_taiko_forks::TaikoHardforks;
mod execute;
pub use execute::*;
mod config;
use config::revm_spec;
use reth_evm_ethereum::revm_spec_by_timestamp_after_merge;

use revm_primitives::{
    BlobExcessGasAndPrice, BlockEnv, Bytes, CfgEnv, Env, HandlerCfg, SpecId, TxKind,
};

/// Optimism-related EVM configuration.
#[derive(Debug, Clone)]
pub struct TaikoEvmConfig {
    chain_spec: Arc<TaikoChainSpec>,
}

impl TaikoEvmConfig {
    /// Creates a new [`TaikoEvmConfig`] with the given chain spec.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { chain_spec }
    }

    /// Returns the chain spec associated with this configuration.
    pub const fn chain_spec(&self) -> &Arc<TaikoChainSpec> {
        &self.chain_spec
    }
}

impl ConfigureEvmEnv for TaikoEvmConfig {
    type Header = Header;
    type Transaction = TransactionSigned;
    type Error = Infallible;

    fn fill_tx_env(
        &self,
        tx_env: &mut TxEnv,
        transaction: &TransactionSigned,
        sender: Address,
        ext: Option<EnvExt<'_>>,
    ) {
        transaction.fill_tx_env(tx_env, sender);

        if let Some(ext) = ext {
            let EnvExt { is_anchor, block_number, extra_data } = ext;
            // Set taiko specific data
            tx_env.taiko.is_anchor = is_anchor;
            if self.chain_spec.is_ontake_active_at_block(block_number) {
                // set the basefee ratio
                tx_env.taiko.basefee_ratio = decode_ontake_extra_data(extra_data);
            }
        }
        // set the treasury address
        let treasury = self.chain_spec.treasury();
        tx_env.taiko.treasury = treasury;
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        env.tx = TxEnv {
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
            taiko: TaikoFields::default(),
        };

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
        let spec_id = revm_spec(
            self.chain_spec(),
            &Head {
                number: header.number,
                timestamp: header.timestamp,
                difficulty: header.difficulty,
                total_difficulty,
                hash: Default::default(),
            },
        );

        cfg_env.chain_id = self.chain_spec.chain().id();
        cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Analyse;

        cfg_env.handler_cfg.spec_id = spec_id;
        cfg_env.handler_cfg.is_taiko = true;
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Self::Error> {
        // configure evm env based on parent block
        let cfg = CfgEnv::default().with_chain_id(self.chain_spec.chain().id());

        // ensure we're not missing any timestamp based hardforks
        let spec_id = revm_spec_by_timestamp_after_merge(&self.chain_spec, attributes.timestamp);

        // if the parent block did not have excess blob gas (i.e. it was pre-cancun), but it is
        // cancun now, we need to set the excess blob gas to the default value(0)
        let blob_excess_gas_and_price = parent
            .next_block_excess_blob_gas()
            .or_else(|| (spec_id.is_enabled_in(SpecId::CANCUN)).then_some(0))
            .map(BlobExcessGasAndPrice::new);

        let block_env = BlockEnv {
            number: U256::from(parent.number + 1),
            coinbase: attributes.suggested_fee_recipient,
            timestamp: U256::from(attributes.timestamp),
            difficulty: U256::ZERO,
            prevrandao: Some(attributes.prev_randao),
            gas_limit: U256::from(parent.gas_limit),
            // calculate basefee based on parent block's gas usage
            basefee: U256::ZERO,
            // calculate excess gas based on parent block's blob gas usage
            blob_excess_gas_and_price,
        };

        let cfg_with_handler_cfg;
        {
            cfg_with_handler_cfg = CfgEnvWithHandlerCfg {
                cfg_env: cfg,
                handler_cfg: HandlerCfg { spec_id, is_taiko: true },
            };
        }

        Ok((cfg_with_handler_cfg, block_env))
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
