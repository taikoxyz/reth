#![allow(missing_docs)]
// We use jemalloc for performance reasons.
#[cfg(all(feature = "jemalloc", unix))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use gwyneth::{engine_api::RpcServerArgsExEx, GwynethNode};
use reth::args::{DiscoveryArgs, NetworkArgs, RpcServerArgs};
use reth_chainspec::ChainSpecBuilder;
use reth_node_builder::{NodeBuilder, NodeConfig, NodeHandle};
use reth_node_ethereum::EthereumNode;
use reth_tasks::TaskManager;

const BASE_CHAIN_ID: u64 = gwyneth::exex::BASE_CHAIN_ID; // Base chain ID for L2s
const NUM_L2_CHAINS: u64 = 2; // Number of L2 chains to create

fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let tasks = TaskManager::current();
        let exec = tasks.executor();
        let network_config = NetworkArgs {
            discovery: DiscoveryArgs { disable_discovery: true, ..DiscoveryArgs::default() },
            ..NetworkArgs::default()
        };

        let mut gwyneth_nodes = Vec::new();

        for i in 0..NUM_L2_CHAINS {
            let chain_id = BASE_CHAIN_ID + (i * 100000); // Increment by 100000 for each L2

            let chain_spec = ChainSpecBuilder::default()
                .chain(chain_id.into())
                .genesis(
                    serde_json::from_str(include_str!(
                        "../../../crates/ethereum/node/tests/assets/genesis.json"
                    ))
                    .unwrap(),
                )
                .cancun_activated()
                .build();

            let node_config = NodeConfig::test()
                .with_chain(chain_spec.clone())
                .with_network(network_config.clone())
                .with_unused_ports()
                .with_rpc(
                    RpcServerArgs::default()
                        .with_unused_ports()
                        .with_static_l2_rpc_ip_and_port(chain_id)
                );

            let NodeHandle { node: gwyneth_node, node_exit_future: _ } =
                NodeBuilder::new(node_config.clone())
                    .gwyneth_node(exec.clone(), chain_spec.chain.id())
                    .node(GwynethNode::default())
                    .launch()
                    .await?;

            gwyneth_nodes.push(gwyneth_node);
        }

        let handle = builder
            .node(EthereumNode::default())
            .install_exex("Rollup", move |ctx| async {
                Ok(gwyneth::exex::Rollup::new(ctx, gwyneth_nodes).await?.start())
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Args, Parser};

    /// A helper type to parse Args more easily
    #[derive(Parser)]
    struct CommandParser<T: Args> {
        #[command(flatten)]
        args: T,
    }
}