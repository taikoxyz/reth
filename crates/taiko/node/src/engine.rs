//! Engine support

use std::sync::Arc;

use alloy_consensus::{BlockHeader, Header};
use alloy_rpc_types_engine::{ExecutionPayloadSidecar, PayloadError};
use reth_node_builder::{
    validate_parent_beacon_block_root_presence, EngineApiMessageVersion,
    EngineObjectValidationError, EngineTypes, EngineValidator, InvalidPayloadAttributesError,
    MessageValidationKind, PayloadAttributes, PayloadOrAttributes, PayloadTypes, PayloadValidator,
    VersionSpecificValidationError,
};
use reth_payload_builder::EthBuiltPayload;
use reth_primitives::{Block, EthereumHardforks, SealedBlock};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_engine_primitives::{
    ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadV1,
    TaikoExecutionPayloadEnvelopeV2, TaikoPayloadBuilderAttributes,
};
use reth_taiko_engine_types::{TaikoExecutionPayload, TaikoPayloadAttributes};
use reth_taiko_payload_validator::TaikoExecutionPayloadValidator;
use reth_tracing::tracing::debug;

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
        debug!(target: "taiko::engine", version=?version, payload_or_attrs=?payload_or_attrs);
        let res = validate_version_specific_fields(self.chain_spec(), version, payload_or_attrs);
        debug!(target: "taiko::engine", version=?version, ?res);
        res
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &TaikoPayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        debug!(target: "taiko::engine", version=?version, attributes=?attributes);
        let res = validate_version_specific_fields(self.chain_spec(), version, attributes.into());
        debug!(target: "taiko::engine", version=?version, ?res);
        res
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

fn validate_withdrawals_presence<T: EthereumHardforks>(
    chain_spec: &T,
    version: EngineApiMessageVersion,
    message_validation_kind: MessageValidationKind,
    timestamp: u64,
    has_withdrawals: bool,
    withdrawals_hash: Option<B256>,
) -> Result<(), EngineObjectValidationError> {
    let is_shanghai_active = chain_spec.is_shanghai_active_at_timestamp(timestamp);

    match version {
        EngineApiMessageVersion::V1 => {
            if has_withdrawals {
                return Err(message_validation_kind
                    .to_error(VersionSpecificValidationError::WithdrawalsNotSupportedInV1))
            }
        }
        EngineApiMessageVersion::V2 | EngineApiMessageVersion::V3 | EngineApiMessageVersion::V4 => {
            if is_shanghai_active && !hash_withdrawals && withdrawals_hash.is_none() {
                return Err(message_validation_kind
                    .to_error(VersionSpecificValidationError::NoWithdrawalsPostShanghai))
            }
            if !is_shanghai_active && has_withdrawals {
                return Err(message_validation_kind
                    .to_error(VersionSpecificValidationError::HasWithdrawalsPreShanghai))
            }
        }
    };

    Ok(())
}

fn validate_version_specific_fields<Type, T>(
    chain_spec: &T,
    version: EngineApiMessageVersion,
    payload_or_attrs: PayloadOrAttributes<'_, Type>,
) -> Result<(), EngineObjectValidationError>
where
    Type: PayloadAttributes,
    T: EthereumHardforks,
{
    let withdrawals_hash = match payload_or_attrs {
        PayloadOrAttributes::ExecutionPayload {
            payload: TaikoExecutionPayload { withdrawals_hash, .. },
            ..
        } => {
            if withdrawals_hash.is_zero() {
                None
            } else {
                Some(*withdrawals_hash)
            }
        }
        _ => None,
    };
    validate_withdrawals_presence(
        chain_spec,
        version,
        payload_or_attrs.message_validation_kind(),
        payload_or_attrs.timestamp(),
        payload_or_attrs.withdrawals().is_some(),
        withdrawals_hash,
    )?;
    validate_parent_beacon_block_root_presence(
        chain_spec,
        version,
        payload_or_attrs.message_validation_kind(),
        payload_or_attrs.timestamp(),
        payload_or_attrs.parent_beacon_block_root().is_some(),
    )
}
