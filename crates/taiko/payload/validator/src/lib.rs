//! Payload Validation support.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use reth_chainspec::{ChainSpec, EthereumHardforks};
use reth_primitives::SealedBlock;
use std::sync::Arc;
use alloy_rpc_types_engine::{MaybeCancunPayloadFields, PayloadError};

/// Execution payload validator.;
#[derive(Clone, Debug)]
pub struct TaikoExecutionPayloadValidator {
    /// Chain spec to validate against.
    chain_spec: Arc<ChainSpec>,
}

impl TaikoExecutionPayloadValidator {
    /// Create a new validator.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }

    /// Returns the chain spec used by the validator.
    #[inline]
    pub fn chain_spec(&self) -> &ChainSpec {
        &self.chain_spec
    }

    /// Returns true if the Cancun hardfork is active at the given timestamp.
    #[inline]
    fn is_cancun_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.chain_spec().is_cancun_active_at_timestamp(timestamp)
    }

    /// Returns true if the Shanghai hardfork is active at the given timestamp.
    #[inline]
    fn is_shanghai_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.chain_spec().is_shanghai_active_at_timestamp(timestamp)
    }

    /// Cancun specific checks for EIP-4844 blob transactions.
    ///
    /// Ensures that the number of blob versioned hashes matches the number hashes included in the
    /// _separate_ `block_versioned_hashes` of the cancun payload fields.
    fn ensure_matching_blob_versioned_hashes(
        &self,
        sealed_block: &SealedBlock,
        cancun_fields: &MaybeCancunPayloadFields,
    ) -> Result<(), PayloadError> {
        todo!()
    }

    /// Ensures that the given payload does not violate any consensus rules that concern the block's
    /// layout, like:
    ///    - missing or invalid base fee
    ///    - invalid extra data
    ///    - invalid transactions
    ///    - incorrect hash
    ///    - the versioned hashes passed with the payload do not exactly match transaction versioned
    ///      hashes
    ///    - the block does not contain blob transactions if it is pre-cancun
    ///
    /// The checks are done in the order that conforms with the engine-API specification.
    ///
    /// This is intended to be invoked after receiving the payload from the CLI.
    /// The additional [`MaybeCancunPayloadFields`] are not part of the payload, but are additional fields in the `engine_newPayloadV3` RPC call, See also <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#engine_newpayloadv3>
    ///
    /// If the cancun fields are provided this also validates that the versioned hashes in the block
    /// match the versioned hashes passed in the
    /// [`CancunPayloadFields`](reth_rpc_types::engine::CancunPayloadFields), if the cancun payload
    /// fields are provided. If the payload fields are not provided, but versioned hashes exist
    /// in the block, this is considered an error: [`PayloadError::InvalidVersionedHashes`].
    ///
    /// This validates versioned hashes according to the Engine API Cancun spec:
    /// <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#specification>
    pub fn ensure_well_formed_payload(
        &self,
        // payload: TaikoExecutionPayload,
        cancun_fields: MaybeCancunPayloadFields,
    ) -> Result<SealedBlock, PayloadError> {
        todo!()
    }
}
