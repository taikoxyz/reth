//! The `L1Origin` module provides the `L1Origin` struct and the `HeadL1Origin` table.
use alloy_primitives::{Address, B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use reth_codecs::Compact;
use reth_db_api::{
    table::{Compress, Decode, Decompress, Encode},
    DatabaseError,
};
use serde::{Deserialize, Serialize};
reth_db_api::impl_compression_for_compact!(L1OriginLegacy);
reth_db_api::impl_compression_for_compact!(L1Origin);

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
        if value == [0] {
            Ok(Self)
        } else {
            Err(DatabaseError::Decode)
        }
    }
}

/// `L1Origin` represents a `L1Origin` of a L2 block.
#[derive(
    Debug,
    Default,
    Compact,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    RlpDecodable,
    RlpEncodable,
)]
#[serde(rename_all = "camelCase")]
#[rlp(trailing)]
pub struct L1Origin {
    /// The block number of the l2 block
    #[serde(rename = "blockID")]
    pub block_id: U256,
    /// The hash of the l2 block
    pub l2_block_hash: B256,
    /// The height of the l1 block
    pub l1_block_height: Option<U256>,
    /// The hash of the l1 block
    pub l1_block_hash: Option<B256>,
    // preconf fields
    /// The batch id of the l1 block
    #[serde(rename = "batchID")]
    pub batch_id: Option<U256>,
    /// The end of block of the l1 block
    pub end_of_block: Option<bool>,
    /// The end of preconf of the l1 block
    pub end_of_preconf: Option<bool>,
    /// The preconfer of the l1 block
    pub preconfer: Option<Address>,
}

impl From<L1OriginLegacy> for L1Origin {
    fn from(legacy: L1OriginLegacy) -> Self {
        Self {
            block_id: legacy.block_id,
            l2_block_hash: legacy.l2_block_hash,
            l1_block_height: Some(legacy.l1_block_height),
            l1_block_hash: Some(legacy.l1_block_hash),
            batch_id: None,
            end_of_block: None,
            end_of_preconf: None,
            preconfer: None,
        }
    }
}

/// `L1OriginLegacy` represents a `L1OriginLegacy` of a L2 block.
#[derive(
    Debug,
    Default,
    Compact,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    RlpDecodable,
    RlpEncodable,
)]
#[serde(rename_all = "camelCase")]
pub struct L1OriginLegacy {
    /// The block number of the l2 block
    #[serde(rename = "blockID")]
    pub block_id: U256,
    /// The hash of the l2 block
    pub l2_block_hash: B256,
    /// The height of the l1 block
    pub l1_block_height: U256,
    /// The hash of the l1 block
    pub l1_block_hash: B256,
}

impl L1Origin {
    /// Returns true if the `L1Origin` is a softblock
    pub const fn is_softblock(&self) -> bool {
        self.batch_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_origin() {
        let val = L1Origin {
            block_id: U256::from(1),
            l2_block_hash: B256::from([2; 32]),
            l1_block_height: Some(U256::from(3)),
            l1_block_hash: Some(B256::from([4; 32])),
            batch_id: None,
            end_of_block: None,
            end_of_preconf: None,
            preconfer: None,
        };
        let val_str = serde_json::to_string(&val).unwrap();

        println!("{}", val_str);
    }
}
