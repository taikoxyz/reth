//! OP-Reth hard forks.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod hardfork;
pub use hardfork::{
    TaikoHardfork, CHAIN_HEKLA_TESTNET, CHAIN_INTERNAL_TESTNET, CHAIN_KATLA_TESTNET, CHAIN_MAINNET,
};
use reth_ethereum_forks::EthereumHardforks;

/// Extends [`EthereumHardforks`] with optimism helper methods.
pub trait TaikoHardforks: EthereumHardforks {
    /// Convenience method to check if [`TaikoHardfork::Hekla`] is active at a given block
    /// number.
    fn is_hekla_active_at_block(&self, block_number: u64) -> bool {
        self.fork(TaikoHardfork::Hekla).active_at_block(block_number)
    }

    /// Convenience method to check if [`TaikoHardfork::Hekla`] is active at a given block
    /// number.
    fn is_ontake_active_at_block(&self, block_number: u64) -> bool {
        self.fork(TaikoHardfork::Ontake).active_at_block(block_number)
    }
}
