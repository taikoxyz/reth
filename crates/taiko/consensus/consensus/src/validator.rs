use std::sync::Arc;
use taiko_reth_chainspec::TaikoChainSpec;
use reth_engine_primitives::{EngineApiMessageVersion, EngineObjectValidationError, EngineTypes, EngineValidator, PayloadOrAttributes, PayloadTypes};
use alloy_rpc_types_engine::PayloadAttributes;
use reth_node_api::validate_version_specific_fields;

/// Validator for Optimism engine API.
#[derive(Debug, Clone)]
pub struct TaikoEngineValidator {
    chain_spec: Arc<TaikoChainSpec>,
}

impl TaikoEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self { chain_spec }
    }
}

impl<Types> EngineValidator<Types> for TaikoEngineValidator
where
    Types: EngineTypes<PayloadAttributes=PayloadAttributes>,
{
    fn validate_version_specific_fields(
        &self,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, PayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(&self.chain_spec, version, payload_or_attrs)
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &PayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(&self.chain_spec, version, attributes.into())
    }
}