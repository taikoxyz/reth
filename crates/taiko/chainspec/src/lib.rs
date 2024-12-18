//! A [Consensus] implementation for local testing purposes
//! that automatically seals blocks.
//!
//! The Mining task polls a [`MiningMode`], and will return a list of transactions that are ready to
//! be mined.
//!
//! These downloaders poll the miner, assemble the block, and return transactions that are ready to
//! be mined.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]

mod spec;
pub use spec::*;
pub mod taiko;
pub use taiko::*;

use alloy_genesis::Genesis;
use derive_more::Into;
use eyre::bail;
use reth_chainspec::ChainSpec;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Clap value parser for [`ChainSpec`]s.
///
/// The value parser matches either a known chain, the path
/// to a json file, or a json formatted string in-memory. The json needs to be a Genesis struct.
pub fn chain_value_parser(s: &str) -> eyre::Result<Arc<ChainSpec>, eyre::Error> {
    Ok(match s {
        "taiko-internal-l2a" => TAIKO_INTERNAL_L2_A.clone(),
        _ => {
            if let Ok(chain_id) = s.parse::<u64>() {
                if let Ok(chain) = TaikoNamedChain::try_from(chain_id) {
                    match chain {
                        TaikoNamedChain::Mainnet => TAIKO_MAINNET.clone(),
                        TaikoNamedChain::TaikoInternalL2a => TAIKO_INTERNAL_L2_A.clone(),
                        TaikoNamedChain::Katla => TAIKO_TESTNET.clone(),
                        TaikoNamedChain::Hekla => TAIKO_HEKLA.clone(),
                        _ => {
                            let genesis = get_taiko_genesis(chain);
                            Arc::new(genesis.into())
                        }
                    }
                } else {
                    bail!("Invalid taiko chain id: {}", chain_id)
                }
            } else {
                // try to read json from path first
                let raw =
                    match fs::read_to_string(PathBuf::from(shellexpand::full(s)?.into_owned())) {
                        Ok(raw) => raw,
                        Err(io_err) => {
                            // valid json may start with "\n", but must contain "{"
                            if s.contains('{') {
                                s.to_string()
                            } else {
                                return Err(io_err.into()); // assume invalid path
                            }
                        }
                    };

                // both serialized Genesis and ChainSpec structs supported
                let genesis: Genesis = serde_json::from_str(&raw)?;

                Arc::new(genesis.into())
            }
        }
    })
}
