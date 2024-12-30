//! Engine support

use std::sync::Arc;

use alloy_consensus::{BlockHeader, Header};
use alloy_rpc_types_engine::{ExecutionPayloadSidecar, PayloadError};
use reth_node_builder::{
    EngineApiMessageVersion, EngineObjectValidationError, EngineTypes, EngineValidator,
    InvalidPayloadAttributesError, PayloadAttributes, PayloadOrAttributes, PayloadTypes,
    PayloadValidator,
};
use reth_payload_builder::EthBuiltPayload;
use reth_payload_primitives::validate_version_specific_fields;
use reth_primitives::{Block, SealedBlock};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_engine_primitives::{
    ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadV1,
    TaikoExecutionPayloadEnvelopeV2, TaikoPayloadBuilderAttributes,
};
use reth_taiko_engine_types::{TaikoExecutionPayload, TaikoPayloadAttributes};
use reth_taiko_payload_validator::TaikoExecutionPayloadValidator;

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
    inner: TaikoExecutionPayloadValidator<TaikoChainSpec>,
}

impl TaikoEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { inner: TaikoExecutionPayloadValidator::new(chain_spec) }
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
        payload: TaikoExecutionPayload,
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

    fn validate_payload_attributes_against_header(
        &self,
        attr: &TaikoPayloadAttributes,
        header: &Header,
    ) -> Result<(), InvalidPayloadAttributesError> {
        if attr.timestamp() < header.timestamp() {
            return Err(InvalidPayloadAttributesError::InvalidTimestamp);
        }
        Ok(())
    }
}
