//! Ethereum specific

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod payload;
use std::sync::Arc;

use alloy_rpc_types_engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError};
pub use alloy_rpc_types_engine::{
    ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4,
    ExecutionPayloadV1, PayloadAttributes as EthPayloadAttributes,
};
pub use payload::{
    TaikoExecutionPayload, TaikoExecutionPayloadEnvelopeV2, TaikoPayloadAttributes,
    TaikoPayloadBuilderAttributes,
};
use reth_engine_primitives::{
    EngineApiMessageVersion, EngineObjectValidationError, EngineTypes, EngineValidator,
    PayloadOrAttributes, PayloadValidator,
};
use reth_ethereum_engine_primitives::EthBuiltPayload;
use reth_payload_primitives::{validate_version_specific_fields, PayloadTypes};
use reth_payload_validator::ExecutionPayloadValidator;
use reth_primitives::{Block, SealedBlock};
use reth_taiko_chainspec::TaikoChainSpec;

/// The types used in the default mainnet ethereum beacon consensus engine.
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub struct TaikoEngineTypes<T: PayloadTypes = TaikoPayloadTypes> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: PayloadTypes> PayloadTypes for TaikoEngineTypes<T> {
    type BuiltPayload = T::BuiltPayload;
    type PayloadAttributes = T::PayloadAttributes;
    type PayloadBuilderAttributes = T::PayloadBuilderAttributes;
}

impl<T: PayloadTypes> EngineTypes for TaikoEngineTypes<T>
where
    T::BuiltPayload: TryInto<ExecutionPayloadV1>
        + TryInto<TaikoExecutionPayloadEnvelopeV2>
        + TryInto<ExecutionPayloadEnvelopeV3>
        + TryInto<ExecutionPayloadEnvelopeV4>,
{
    type ExecutionPayloadEnvelopeV1 = ExecutionPayloadV1;
    type ExecutionPayloadEnvelopeV2 = TaikoExecutionPayloadEnvelopeV2;
    type ExecutionPayloadEnvelopeV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadEnvelopeV4 = ExecutionPayloadEnvelopeV4;
}

/// A default payload type for [`EthEngineTypes`]
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub struct TaikoPayloadTypes;

impl PayloadTypes for TaikoPayloadTypes {
    type BuiltPayload = EthBuiltPayload;
    type PayloadAttributes = TaikoPayloadAttributes;
    type PayloadBuilderAttributes = TaikoPayloadBuilderAttributes;
}

/// Validator for the ethereum engine API.
#[derive(Debug, Clone)]
pub struct TaikoEngineValidator {
    inner: ExecutionPayloadValidator<TaikoChainSpec>,
}

impl TaikoEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { inner: ExecutionPayloadValidator::new(chain_spec) }
    }

    /// Returns the chain spec used by the validator.
    #[inline]
    fn chain_spec(&self) -> &TaikoChainSpec {
        self.inner.chain_spec()
    }
}

impl PayloadValidator for TaikoEngineValidator {
    type Block = Block;

    fn ensure_well_formed_payload(
        &self,
        payload: ExecutionPayload,
        sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlock, PayloadError> {
        self.inner.ensure_well_formed_payload(payload, sidecar)
    }
}

impl<Types> EngineValidator<Types> for TaikoEngineValidator
where
    Types: EngineTypes<PayloadAttributes = TaikoPayloadAttributes>,
{
    fn validate_version_specific_fields(
        &self,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, TaikoPayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, payload_or_attrs)
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &TaikoPayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, attributes.into())
    }
}
