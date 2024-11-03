use alloy_primitives::{B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use reth_db_api::DatabaseError;
use reth_db_api::table::{Decode, Encode};
// TODO: fix impl_compression_for_compact
// reth_db_api::impl_compression_for_compact!(L1Origin);

/// The key for the latest l1 origin
#[derive(Debug, Default, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct HeadL1OriginKey;

impl Encode for HeadL1OriginKey {
    type Encoded = [u8; 1];

    fn encode(self) -> Self::Encoded {
        [0]
    }
}

impl Decode for HeadL1OriginKey {
    fn decode(value: &[u8]) -> Result<Self, DatabaseError> {
        if value.as_ref() == [0] {
            Ok(Self)
        } else {
            Err(DatabaseError::Decode)
        }
    }
}

/// L1Origin represents a L1Origin of a L2 block.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, RlpDecodable, RlpEncodable, Serialize, Deserialize)]
#[rlp(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct L1Origin {
    /// The block number of the l2 block
    #[rlp(rename = "blockID")]
    #[serde(rename = "blockID")]
    pub block_id: U256,
    /// The hash of the l2 block
    pub l2_block_hash: B256,
    /// The height of the l1 block
    pub l1_block_height: U256,
    /// The hash of the l1 block
    pub l1_block_hash: B256,
}