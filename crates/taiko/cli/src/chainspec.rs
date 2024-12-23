use reth_cli::chainspec::{parse_genesis, ChainSpecParser};
use reth_taiko_chainspec::{get_taiko_genesis, TaikoChainSpec};
use reth_taiko_forks::{CHAIN_HEKLA_TESTNET, CHAIN_INTERNAL_TESTNET, CHAIN_MAINNET};
use std::sync::Arc;

/// Optimism chain specification parser.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TaikoChainSpecParser;

impl ChainSpecParser for TaikoChainSpecParser {
    type ChainSpec = TaikoChainSpec;

    const SUPPORTED_CHAINS: &'static [&'static str] =
        &["hekla", "internal-a", "mainnet", "testnet"];

    fn parse(s: &str) -> eyre::Result<Arc<Self::ChainSpec>> {
        chain_value_parser(s)
    }
}

/// Clap value parser for [`OpChainSpec`]s.
///
/// The value parser matches either a known chain, the path
/// to a json file, or a json formatted string in-memory. The json needs to be a Genesis struct.
pub fn chain_value_parser(s: &str) -> eyre::Result<Arc<TaikoChainSpec>, eyre::Error> {
    Ok(Arc::new(match s {
        "hekla" => get_taiko_genesis(CHAIN_HEKLA_TESTNET).into(),
        "internal-a" => get_taiko_genesis(CHAIN_INTERNAL_TESTNET).into(),
        "mainnet" => get_taiko_genesis(CHAIN_MAINNET).into(),
        _ => parse_genesis(s)?.into(),
    }))
}
