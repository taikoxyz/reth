//! Payload related types

use alloy_eips::eip4895::Withdrawal;
use alloy_primitives::{Address, Bytes, PrimitiveSignature as Signature, B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadInputV2, ExecutionPayloadV2, PayloadAttributes,
};
use reth_taiko_primitives::L1Origin;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

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

/// This structure contains the information from l1 contract storage
#[serde_as]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable,
)]
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

/// Taiko Execution Payload Envelope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoExecutionPayloadEnvelopeV2 {
    /// Taiko execution payload
    pub execution_payload: TaikoExecutionPayloadV2,
    /// The expected value to be received by the feeRecipient in wei
    pub block_value: U256,
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

    /// Returns the withdrawals
    pub const fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        self.payload_inner.withdrawals()
    }

    /// Returns the timestamp
    pub const fn timestamp(&self) -> u64 {
        self.payload_inner.timestamp()
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

/// This is the input to `engine_newPayloadV2`, which may or may not have a withdrawals field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaikoExecutionPayloadInputV2 {
    /// The V1 execution payload
    #[serde(flatten)]
    pub execution_payload: ExecutionPayloadInputV2,
    /// Allow passing txHash directly instead of transactions list
    pub tx_hash: B256,
    /// Allow passing `WithdrawalsHash` directly instead of withdrawals
    pub withdrawals_hash: B256,
    /// Blob gas used
    #[serde(with = "alloy_serde::quantity::opt")]
    pub blob_gas_used: Option<u64>,
    /// Excess blob gas
    #[serde(with = "alloy_serde::quantity::opt")]
    pub excess_blob_gas: Option<u64>,
    /// Deposit requests
    pub deposit_requests: Option<Vec<TxDeposit>>,

    /// Is taiko block
    #[serde(rename = "TaikoBlock")]
    pub taiko_block: bool,
}

/// TxDeposit
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxDeposit {
    /// Public key
    pub public_key: PublicKey,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount
    pub amount: u64,
    /// Signature
    pub signature: Signature,
    /// Index
    pub index: u64,
}
