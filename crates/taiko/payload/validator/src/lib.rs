//! Payload Validation support.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use alloy_rpc_types_engine::{ExecutionPayloadSidecar, PayloadError};
use reth_chainspec::EthereumHardforks;
use reth_payload_validator::ExecutionPayloadValidator;
use reth_primitives::SealedBlock;
use reth_taiko_engine_primitives::TaikoExecutionPayload;
use std::{ops::Deref, sync::Arc};

/// Execution payload validator.;
#[derive(Clone, Debug)]
pub struct TaikoExecutionPayloadValidator<ChainSpec> {
    /// Chain spec to validate against.
    inner: ExecutionPayloadValidator<ChainSpec>,
}

impl<ChainSpec> Deref for TaikoExecutionPayloadValidator<ChainSpec> {
    type Target = ExecutionPayloadValidator<ChainSpec>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<ChainSpec: EthereumHardforks> TaikoExecutionPayloadValidator<ChainSpec> {
    /// Create a new validator.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { inner: ExecutionPayloadValidator::new(chain_spec) }
    }

    /// Call the inner validator to ensure the payload is well formed.
    /// Transactions and withdrawals roots are set from the payload.
    pub fn ensure_well_formed_payload(
        &self,
        payload: TaikoExecutionPayload,
        sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlock, PayloadError> {
        let TaikoExecutionPayload { payload_inner, tx_hash, withdrawals_hash } = payload;
        let mut block = self.inner.ensure_well_formed_payload(payload_inner, sidecar)?;
        block.body = Default::default();
        block.transactions_root = tx_hash;
        block.withdrawals_root = Some(withdrawals_hash);
        Ok(block)
    }
}
