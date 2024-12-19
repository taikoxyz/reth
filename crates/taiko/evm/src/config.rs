use reth_chainspec::{ChainSpec, Head};
use reth_taiko_forks::TaikoHardfork;

pub(crate) fn revm_spec(chain_spec: &ChainSpec, block: &Head) -> revm_primitives::SpecId {
    if chain_spec.fork(TaikoHardfork::Ontake).active_at_head(block) {
        revm_primitives::ONTAKE
    } else {
        reth_evm_ethereum::revm_spec(chain_spec, block)
    }
}
