//! Hard forks of taiko protocol.

use alloy_primitives::U256;
use core::{
    any::Any,
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use reth_ethereum_forks::{hardfork, ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};

hardfork!(
    /// The name of an Taiko hardfork.
    TaikoHardFork {
    /// Hekla: the 1st taiko mainnet version: <>
    Hekla,
    /// Ontake: the 2nd taiko mainnet fork: <>
    Ontake,
});

impl TaikoHardFork {
    /// Taiko taiko-internal-l2a list of hardforks.
    pub fn taiko_internal_l2a() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::Paris.boxed(),
                ForkCondition::TTD { fork_block: None, total_difficulty: U256::from(0) },
            ),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (TaikoHardFork::Hekla.boxed(), ForkCondition::Block(0)),
            (TaikoHardFork::Ontake.boxed(), ForkCondition::Block(2)), //todo
        ])
    }

    // Taiko taiko-testnet list of hardforks.
    pub fn taiko_testnet() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::Paris.boxed(),
                ForkCondition::TTD { fork_block: None, total_difficulty: U256::from(0) },
            ),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (Self::Hekla.boxed(), ForkCondition::Block(0)),
        ])
    }

    // Taiko taiko-hekla list of hardforks.
    pub fn taiko_hekla() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::Paris.boxed(),
                ForkCondition::TTD { fork_block: None, total_difficulty: U256::from(0) },
            ),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (Self::Hekla.boxed(), ForkCondition::Block(0)),
            (Self::Ontake.boxed(), ForkCondition::Block(720_000)), //todo
        ])
    }

    pub fn taiko_mainnet() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(1561651)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(4460644)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(5062605)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::Paris.boxed(),
                ForkCondition::TTD { fork_block: None, total_difficulty: U256::from(0) },
            ),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (Self::Hekla.boxed(), ForkCondition::Block(0)),
            (Self::Ontake.boxed(), ForkCondition::Block(374_400)), //todo
        ])
    }
}

/// Match helper method since it's not possible to match on `dyn Hardfork`
fn match_hardfork<H, HF, OHF>(fork: H, hardfork_fn: HF, optimism_hardfork_fn: OHF) -> Option<u64>
where
    H: Hardfork,
    HF: Fn(&EthereumHardfork) -> Option<u64>,
    OHF: Fn(&TaikoHardFork) -> Option<u64>,
{
    let fork: &dyn Any = &fork;
    if let Some(fork) = fork.downcast_ref::<EthereumHardfork>() {
        return hardfork_fn(fork);
    }
    fork.downcast_ref::<TaikoHardFork>().and_then(optimism_hardfork_fn)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn check_op_hardfork_from_str() {
        let hardfork_str = ["Hekla", "Ontake"];
        let expected_hardforks = [TaikoHardFork::Hekla, TaikoHardFork::Ontake];

        let hardforks: Vec<TaikoHardFork> =
            hardfork_str.iter().map(|h| TaikoHardFork::from_str(h).unwrap()).collect();

        assert_eq!(hardforks, expected_hardforks);
    }

    #[test]
    fn check_nonexistent_hardfork_from_str() {
        assert!(TaikoHardFork::from_str("not a hardfork").is_err());
    }
}
