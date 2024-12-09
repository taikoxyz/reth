use std::sync::Arc;
use alloy_rpc_types_engine::{ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadV1, ExecutionPayloadV2};
use reth_chainspec::ChainSpec;
use reth_node_api::{EngineApiMessageVersion, EngineObjectValidationError, EngineTypes, EngineValidator, PayloadOrAttributes, PayloadTypes};
use taiko_reth_engine_primitives::{TaikoBuiltPayload, TaikoExecutionPayloadEnvelopeV2, TaikoPayloadAttributes, TaikoPayloadBuilderAttributes};

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
    T::BuiltPayload:
    TryInto<ExecutionPayloadV1>
    + TryInto<TaikoExecutionPayloadEnvelopeV2>
    + TryInto<ExecutionPayloadEnvelopeV3>
    + TryInto<ExecutionPayloadEnvelopeV4>,
{
    type ExecutionPayloadV1 = ExecutionPayloadV1;
    type ExecutionPayloadV2 = TaikoExecutionPayloadEnvelopeV2;
    type ExecutionPayloadV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadV4 = ExecutionPayloadEnvelopeV4;
}

/// A default payload type for [`TaikoEngineTypes`]
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub struct TaikoPayloadTypes;

impl PayloadTypes for TaikoPayloadTypes {
    type BuiltPayload = TaikoBuiltPayload;
    type PayloadAttributes = TaikoPayloadAttributes;
    type PayloadBuilderAttributes = TaikoPayloadBuilderAttributes;
}

/// Validator for Optimism engine API.
#[derive(Debug, Clone)]
pub struct TaikoEngineValidator {
    chain_spec: Arc<ChainSpec>,
}

impl TaikoEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }
}

impl<Types> EngineValidator<Types> for TaikoEngineValidator
where
    Types: EngineTypes<PayloadAttributes=TaikoPayloadAttributes>,
{
    fn validate_version_specific_fields(&self, version: EngineApiMessageVersion, payload_or_attrs: PayloadOrAttributes<'_, <Types as PayloadTypes>::PayloadAttributes>) -> Result<(), EngineObjectValidationError> {
        todo!()
    }

    fn ensure_well_formed_attributes(&self, version: EngineApiMessageVersion, attributes: &<Types as PayloadTypes>::PayloadAttributes) -> Result<(), EngineObjectValidationError> {
        todo!()
    }
}