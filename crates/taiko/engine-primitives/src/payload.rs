//! Payload related types

use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadV1,
    ExecutionPayloadV2, PayloadAttributes, PayloadId,
};
use reth_ethereum_engine_primitives::EthPayloadBuilderAttributes;
use reth_payload_primitives::{BuiltPayload, EngineApiMessageVersion, PayloadBuilderAttributes};
use reth_primitives::revm_primitives::{Address, Bytes, B256, U256};
use reth_primitives::{BlobTransactionSidecar, SealedBlock, Withdrawals};
use reth_rpc_types_compat::engine::block_to_payload_v1;
use reth_rpc_types_compat::engine::payload::{
    block_to_payload_v2, block_to_payload_v3, block_to_payload_v4,
};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use std::convert::Infallible;
use taiko_reth_primitives::L1Origin;

/// Contains the built payload.
#[derive(Debug, Clone)]
pub struct TaikoBuiltPayload {
    /// Identifier of the payload
    pub(crate) id: PayloadId,
    /// The built block
    pub(crate) block: SealedBlock,
    /// The fees of the block
    pub(crate) fees: U256,
    /// The blobs, proofs, and commitments in the block. If the block is pre-cancun, this will be
    /// empty.
    pub(crate) sidecars: Vec<BlobTransactionSidecar>,
}

impl BuiltPayload for TaikoBuiltPayload {
    fn block(&self) -> &SealedBlock {
        &self.block
    }

    fn fees(&self) -> U256 {
        self.fees
    }
}

impl<'a> BuiltPayload for &'a TaikoBuiltPayload {
    fn block(&self) -> &SealedBlock {
        (**self).block()
    }

    fn fees(&self) -> reth_primitives::revm_primitives::ruint::aliases::U256 {
        (**self).fees()
    }
}

// V1 engine_getPayloadV1 response
impl From<TaikoBuiltPayload> for ExecutionPayloadV1 {
    fn from(value: TaikoBuiltPayload) -> Self {
        block_to_payload_v1(value.block)
    }
}

impl From<TaikoBuiltPayload> for ExecutionPayloadEnvelopeV3 {
    fn from(value: TaikoBuiltPayload) -> Self {
        let TaikoBuiltPayload { block, fees, sidecars, .. } = value;

        Self {
            execution_payload: block_to_payload_v3(block),
            block_value: fees,
            // From the engine API spec:
            //
            // > Client software **MAY** use any heuristics to decide whether to set
            // `shouldOverrideBuilder` flag or not. If client software does not implement any
            // heuristic this flag **SHOULD** be set to `false`.
            //
            // Spec:
            // <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#specification-2>
            should_override_builder: false,
            blobs_bundle: sidecars.into_iter().map(Into::into).collect::<Vec<_>>().into(),
        }
    }
}

impl From<TaikoBuiltPayload> for ExecutionPayloadEnvelopeV4 {
    fn from(value: TaikoBuiltPayload) -> Self {
        let TaikoBuiltPayload { block, fees, sidecars, .. } = value;

        Self {
            execution_payload: block_to_payload_v4(block),
            block_value: fees,
            // From the engine API spec:
            //
            // > Client software **MAY** use any heuristics to decide whether to set
            // `shouldOverrideBuilder` flag or not. If client software does not implement any
            // heuristic this flag **SHOULD** be set to `false`.
            //
            // Spec:
            // <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#specification-2>
            should_override_builder: false,
            blobs_bundle: sidecars.into_iter().map(Into::into).collect::<Vec<_>>().into(),
        }
    }
}

/// Taiko Payload Attributes
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoPayloadAttributes {
    /// The payload attributes
    #[serde(flatten)]
    pub payload_attributes: PayloadAttributes,
    /// EIP1559 base fee
    pub base_fee_per_gas: U256,
    /// Data from l1 contract
    pub block_metadata: BlockMetadata,
    /// l1 anchor information
    pub l1_origin: L1Origin,
}

impl reth_payload_primitives::PayloadAttributes for TaikoPayloadAttributes {
    fn timestamp(&self) -> u64 {
        self.payload_attributes.timestamp
    }

    fn withdrawals(&self) -> Option<&Vec<alloy_eips::eip4895::Withdrawal>> {
        self.payload_attributes.withdrawals.as_ref()
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.payload_attributes.parent_beacon_block_root
    }
}

/// This structure contains the information from l1 contract storage
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
#[serde(rename_all = "camelCase")]
pub struct BlockMetadata {
    /// The Keccak 256-bit hash of the parent
    /// block’s header, in its entirety; formally Hp.
    pub beneficiary: Address,
    /// A scalar value equal to the current limit of gas expenditure per block; formally Hl.
    pub gas_limit: u64,
    /// Timestamp in l1
    #[serde(with = "alloy_serde::quantity")]
    pub timestamp: u64,
    /// A 256-bit hash which, combined with the
    /// nonce, proves that a sufficient amount of computation has been carried out on this block;
    /// formally Hm.
    pub mix_hash: B256,
    /// The origin transactions data
    pub tx_list: Bytes,
    /// An arbitrary byte array containing data relevant to this block. This must be 32 bytes or
    /// fewer; formally Hx.
    #[serde_as(as = "Base64")]
    pub extra_data: Vec<u8>,
}

/// Taiko Payload Builder Attributes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaikoPayloadBuilderAttributes {
    /// Inner ethereum payload builder attributes
    pub payload_attributes: EthPayloadBuilderAttributes,
    /// The base layer fee per gas
    pub base_fee_per_gas: reth_primitives::revm_primitives::ruint::aliases::U256,
    /// Taiko specific block metadata
    pub block_metadata: BlockMetadata,
    /// The L1 origin of the L2 block
    pub l1_origin: L1Origin,
}

impl PayloadBuilderAttributes for TaikoPayloadBuilderAttributes {
    type RpcPayloadAttributes = TaikoPayloadAttributes;
    type Error = Infallible;

    fn try_new(
        parent: B256,
        attributes: TaikoPayloadAttributes,
        version: EngineApiMessageVersion,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let id = payload_id(&parent, &attributes, version);

        let payload_attributes = EthPayloadBuilderAttributes {
            id,
            parent,
            timestamp: attributes.payload_attributes.timestamp,
            suggested_fee_recipient: attributes.payload_attributes.suggested_fee_recipient,
            prev_randao: attributes.payload_attributes.prev_randao,
            withdrawals: attributes.payload_attributes.withdrawals.unwrap_or_default().into(),
            parent_beacon_block_root: attributes.payload_attributes.parent_beacon_block_root,
        };

        Ok(Self {
            payload_attributes,
            base_fee_per_gas: attributes.base_fee_per_gas,
            block_metadata: attributes.block_metadata,
            l1_origin: attributes.l1_origin,
        })
    }

    fn payload_id(&self) -> alloy_rpc_types_engine::payload::PayloadId {
        self.payload_attributes.id
    }

    fn parent(&self) -> B256 {
        self.payload_attributes.parent
    }

    fn timestamp(&self) -> u64 {
        self.payload_attributes.timestamp
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.payload_attributes.parent_beacon_block_root
    }

    fn suggested_fee_recipient(&self) -> Address {
        self.payload_attributes.suggested_fee_recipient
    }

    fn prev_randao(&self) -> B256 {
        self.payload_attributes.prev_randao
    }

    fn withdrawals(&self) -> &Withdrawals {
        &self.payload_attributes.withdrawals
    }
}

/// Taiko Execution Payload
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoExecutionPayloadV2 {
    /// Inner V2 payload
    #[serde(flatten)]
    pub payload_inner: ExecutionPayloadV2,

    /// Allow passing txHash directly instead of transactions list
    pub tx_hash: B256,
    /// Allow passing withdrawals hash directly instead of withdrawals
    pub withdrawals_hash: B256,
}

impl From<ExecutionPayloadV2> for TaikoExecutionPayloadV2 {
    fn from(value: ExecutionPayloadV2) -> Self {
        Self { payload_inner: value, tx_hash: B256::default(), withdrawals_hash: B256::default() }
    }
}

impl From<TaikoBuiltPayload> for TaikoExecutionPayloadV2 {
    fn from(value: TaikoBuiltPayload) -> Self {
        let TaikoBuiltPayload { block, .. } = value;

        Self {
            tx_hash: block.header.transactions_root,
            withdrawals_hash: block.header.withdrawals_root.unwrap_or_default(),
            payload_inner: block_to_payload_v2(block),
        }
    }
}

/// Taiko Execution Payload Envelope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoExecutionPayloadEnvelopeV2 {
    /// Taiko execution payload
    pub execution_payload: TaikoExecutionPayloadV2,
    /// The expected value to be received by the feeRecipient in wei
    pub block_value: reth_primitives::revm_primitives::ruint::aliases::U256,
}

impl From<TaikoBuiltPayload> for TaikoExecutionPayloadEnvelopeV2 {
    fn from(value: TaikoBuiltPayload) -> Self {
        let fees = value.fees;
        Self { execution_payload: value.into(), block_value: fees }
    }
}

/// An tiako execution payload
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoExecutionPayload {
    /// Inner V3 payload
    #[serde(flatten)]
    pub payload_inner: ExecutionPayload,

    /// Allow passing txHash directly instead of transactions list
    pub tx_hash: B256,
    /// Allow passing `WithdrawalsHash` directly instead of withdrawals
    pub withdrawals_hash: B256,
}

impl TaikoExecutionPayload {
    /// Returns the block hash
    pub const fn block_hash(&self) -> B256 {
        self.payload_inner.block_hash()
    }

    /// Returns the block number
    pub const fn block_number(&self) -> u64 {
        self.payload_inner.block_number()
    }

    /// Returns the parent hash
    pub const fn parent_hash(&self) -> B256 {
        self.payload_inner.parent_hash()
    }
}

impl From<(ExecutionPayload, B256, B256)> for TaikoExecutionPayload {
    fn from((payload_inner, tx_hash, withdrawals_hash): (ExecutionPayload, B256, B256)) -> Self {
        Self { payload_inner, tx_hash, withdrawals_hash }
    }
}

impl From<ExecutionPayload> for TaikoExecutionPayload {
    fn from(value: ExecutionPayload) -> Self {
        Self { payload_inner: value, tx_hash: B256::default(), withdrawals_hash: B256::default() }
    }
}

/// Generates the payload id for the configured payload from the [`PayloadAttributes`].
///
/// Returns an 8-byte identifier by hashing the payload components with sha256 hash.
pub(crate) fn payload_id(
    parent: &B256,
    attributes: &TaikoPayloadAttributes,
    version: EngineApiMessageVersion,
) -> PayloadId {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(parent.as_slice());
    hasher.update(&attributes.payload_attributes.timestamp.to_be_bytes()[..]);
    hasher.update(attributes.payload_attributes.prev_randao.as_slice());
    hasher.update(attributes.payload_attributes.suggested_fee_recipient.as_slice());
    if let Some(withdrawals) = &attributes.payload_attributes.withdrawals {
        let mut buf = Vec::new();
        withdrawals.encode(&mut buf);
        hasher.update(buf);
    }

    if let Some(parent_beacon_block) = attributes.payload_attributes.parent_beacon_block_root {
        hasher.update(parent_beacon_block);
    }

    hasher.update(attributes.base_fee_per_gas.to_be_bytes_vec());

    let mut buf = Vec::new();
    attributes.block_metadata.encode(&mut buf);
    hasher.update(buf);

    let mut buf = Vec::new();
    attributes.l1_origin.encode(&mut buf);
    hasher.update(buf);

    let out = hasher.finalize();
    let mut out_bytes: [u8; 8] = out.as_slice()[..8].try_into().expect("sufficient length");
    out_bytes[0] = version as u8;
    PayloadId::new(out_bytes)
}
