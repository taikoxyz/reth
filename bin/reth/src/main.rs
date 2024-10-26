#![allow(missing_docs)]
// We use jemalloc for performance reasons.
#[cfg(all(feature = "jemalloc", unix))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::sync::Arc;

use gwyneth::{engine_api::RpcServerArgsExEx, GwynethNode};
use reth::args::{DiscoveryArgs, NetworkArgs, RpcServerArgs};
use reth_chainspec::ChainSpecBuilder;
use reth_cli_commands::node::L2Args;
use reth_db::init_db;
use reth_node_builder::{NodeBuilder, NodeConfig, NodeHandle};
use reth_node_ethereum::EthereumNode;
use reth_tasks::TaskManager;

fn main() -> eyre::Result<()> {
    println!("WTF");
    reth::cli::Cli::<L2Args>::parse_args_l2().run(|builder, ext| async move {
        println!("Starting reth node with custom exex \n {:?}", ext);
        let tasks = TaskManager::current();
        let exec = tasks.executor();
        let network_config = NetworkArgs {
            discovery: DiscoveryArgs { disable_discovery: true, ..DiscoveryArgs::default() },
            ..NetworkArgs::default()
        };

        let mut gwyneth_nodes = Vec::new();


        // Assuming chain_ids & datadirs are mandetory
        // If ports and ipc are not supported we used the default ways to derive 
        assert_eq!(ext.chain_ids.len(), ext.datadirs.len());
        assert!(ext.chain_ids.len() > 0);

        for (idx, (chain_id, datadir)) in ext.chain_ids.into_iter().zip(ext.datadirs).enumerate() {
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
                        .with_ports_and_ipc(ext.ports.get(idx), ext.ipc_path.clone(), chain_id)
                );
            
            let db = Arc::new(init_db(datadir, reth_db::mdbx::DatabaseArguments::default())?);

            let NodeHandle { node: gwyneth_node, node_exit_future: _ } =
                NodeBuilder::new(node_config.clone())
                    .with_database(db)
                    .with_launch_context(exec.clone())
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