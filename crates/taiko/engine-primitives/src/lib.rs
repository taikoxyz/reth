pub mod payload;

use alloy_rpc_types_engine::{ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadV1};
pub use payload::*;
use reth_engine_primitives::EngineTypes;
use reth_payload_primitives::PayloadTypes;

/// The types used in the default mainnet ethereum beacon consensus engine.
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub struct TaikoEngineTypes;

impl PayloadTypes for TaikoEngineTypes {
    type BuiltPayload = TaikoBuiltPayload;
    type PayloadAttributes = TaikoPayloadAttributes;
    type PayloadBuilderAttributes = TaikoPayloadBuilderAttributes;
}

impl EngineTypes for TaikoEngineTypes {
    type ExecutionPayloadV1 = ExecutionPayloadV1;
    type ExecutionPayloadV2 = TaikoExecutionPayloadEnvelopeV2;
    type ExecutionPayloadV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadV4 = ExecutionPayloadEnvelopeV4;
}