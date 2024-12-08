use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use reth_primitives::revm_primitives::{B256, U256};

/// L1Origin represents a L1Origin of a L2 block.
// #[main_codec]    // TODO fix main_codec
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, RlpDecodable, RlpEncodable)]
// #[serde(rename_all = "camelCase")]
pub struct L1Origin {
    /// The block number of the l2 block
    // #[serde(rename = "blockID")]
    pub block_id: U256,
    /// The hash of the l2 block
    pub l2_block_hash: B256,
    /// The height of the l1 block
    pub l1_block_height: U256,
    /// The hash of the l1 block
    pub l1_block_hash: B256,
}