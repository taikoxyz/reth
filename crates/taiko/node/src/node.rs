//! Ethereum Node types config.

use crate::engine::TaikoEngineTypes;
use crate::TaikoEngineValidator;
use reth_auto_seal_consensus::AutoSealConsensus;
use reth_basic_payload_builder::{BasicPayloadJobGenerator, BasicPayloadJobGeneratorConfig};
use reth_chainspec::{ChainSpec, EthChainSpec, Hardforks};
use reth_ethereum_payload_builder::EthereumPayloadBuilder;
use reth_evm::ConfigureEvm;
use reth_network::{NetworkConfig, NetworkHandle, NetworkManager};
use reth_node_api::{EngineValidator, FullNodeComponents, NodeAddOns};
use reth_node_builder::{
    components::{
        ComponentsBuilder, ConsensusBuilder, EngineValidatorBuilder, ExecutorBuilder,
        NetworkBuilder, PayloadServiceBuilder, PoolBuilder, PoolBuilderConfigOverrides,
    },
    node::{FullNodeTypes, NodeTypes, NodeTypesWithEngine},
    BuilderContext, Node, PayloadBuilderConfig,
};
use reth_payload_builder::{PayloadBuilderHandle, PayloadBuilderService};
use reth_primitives::Header;
use reth_provider::CanonStateSubscriptions;
use reth_rpc::EthApi;
use reth_tracing::tracing::{debug, info};
use reth_transaction_pool::{
    blobstore::DiskFileBlobStore, CoinbaseTipOrdering, EthTransactionPool, TransactionPool,
    TransactionValidationTaskExecutor,
};
use std::sync::Arc;
use taiko_reth_beacon_consensus::TaikoBeaconConsensus;
use taiko_reth_evm::{TaikoEvmConfig, TaikoExecutorProvider};

/// Type configuration for a regular Ethereum node.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoNode;

impl TaikoNode {
    /// Returns a [`ComponentsBuilder`] configured for a regular Ethereum node.
    pub fn components<Node>() -> ComponentsBuilder<
        Node,
        TaikoPoolBuilder,
        TaikoPayloadBuilder,
        TaikoNetworkBuilder,
        TaikoExecutorBuilder,
        TaikoConsensusBuilder,
        TaikoEngineValidatorBuilder,
    >
    where
        Node: FullNodeTypes<
            Types: NodeTypesWithEngine<Engine = TaikoEngineTypes, ChainSpec = ChainSpec>,
        >,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(TaikoPoolBuilder::default())
            .payload(TaikoPayloadBuilder::default())
            .network(TaikoNetworkBuilder::default())
            .executor(TaikoExecutorBuilder::default())
            .consensus(TaikoConsensusBuilder::default())
            .engine_validator(TaikoEngineValidatorBuilder::default())
    }
}

impl<N> Node<N> for TaikoNode
where
    N: FullNodeTypes<Types: NodeTypesWithEngine<Engine = TaikoEngineTypes, ChainSpec = ChainSpec>>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        TaikoPoolBuilder,
        TaikoPayloadBuilder,
        TaikoNetworkBuilder,
        TaikoExecutorBuilder,
        TaikoConsensusBuilder,
        TaikoEngineValidatorBuilder,
    >;

    type AddOns = TaikoAddOns;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }

    fn add_ons(&self) -> Self::AddOns {
        TaikoAddOns::default()
    }
}

impl NodeTypes for TaikoNode {
    type Primitives = ();
    type ChainSpec = ChainSpec;
}
impl NodeTypesWithEngine for TaikoNode {
    type Engine = TaikoEngineTypes;
}

/// Add-ons w.r.t. optimism.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TaikoAddOns;

impl<N: FullNodeComponents> NodeAddOns<N> for TaikoAddOns {
    type EthApi = EthApi<N::Provider, N::Pool, NetworkHandle, N::Evm>;
}

/// A regular ethereum evm and executor builder.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for TaikoExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
{
    type EVM = TaikoEvmConfig;
    type Executor = TaikoExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let chain_spec = ctx.chain_spec();
        let evm_config = TaikoEvmConfig::default();
        let executor = TaikoExecutorProvider::new(chain_spec, evm_config.clone());

        Ok((evm_config, executor))
    }
}

/// A basic ethereum transaction pool.
///
/// This contains various settings that can be configured and take precedence over the node's
/// config.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoPoolBuilder;

impl<Node> PoolBuilder<Node> for TaikoPoolBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
{
    type Pool = EthTransactionPool<Node::Provider, DiskFileBlobStore>;

    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let pool_config = ctx.pool_config();
        let blob_store = DiskFileBlobStore::open(data_dir.blobstore(), Default::default())?;
        let validator = TransactionValidationTaskExecutor::eth_builder(ctx.chain_spec())
            .with_head_timestamp(ctx.head().timestamp)
            .kzg_settings(ctx.kzg_settings()?)
            .with_local_transactions_config(pool_config.local_transactions_config.clone())
            .with_additional_tasks(1)
            .build_with_tasks(
                ctx.provider().clone(),
                ctx.task_executor().clone(),
                blob_store.clone(),
            );

        let transaction_pool =
            reth_transaction_pool::Pool::eth_pool(validator, blob_store, pool_config);
        info!(target: "reth::cli", "Transaction pool initialized");
        let transactions_path = data_dir.txpool_transactions();

        // spawn txpool maintenance task
        {
            let pool = transaction_pool.clone();
            let chain_events = ctx.provider().canonical_state_stream();
            let client = ctx.provider().clone();
            let transactions_backup_config =
                reth_transaction_pool::maintain::LocalTransactionBackupConfig::with_local_txs_backup(transactions_path);

            ctx.task_executor().spawn_critical_with_graceful_shutdown_signal(
                "local transactions backup task",
                |shutdown| {
                    reth_transaction_pool::maintain::backup_local_transactions_task(
                        shutdown,
                        pool.clone(),
                        transactions_backup_config,
                    )
                },
            );

            // spawn the maintenance task
            ctx.task_executor().spawn_critical(
                "txpool maintenance task",
                reth_transaction_pool::maintain::maintain_transaction_pool_future(
                    client,
                    pool,
                    chain_events,
                    ctx.task_executor().clone(),
                    Default::default(),
                ),
            );
            debug!(target: "reth::cli", "Spawned txpool maintenance task");
        }

        Ok(transaction_pool)
    }
}

/// A basic ethereum payload service.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct TaikoPayloadBuilder;

impl<Node, Pool> PayloadServiceBuilder<Node, Pool> for TaikoPayloadBuilder
where
    Node:
        FullNodeTypes<Types: NodeTypesWithEngine<Engine = TaikoEngineTypes, ChainSpec = ChainSpec>>,
    Pool: TransactionPool + Unpin + 'static,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<TaikoEngineTypes>> {
        let chain_spec = ctx.chain_spec();
        let evm_config = TaikoEvmConfig::default();
        let payload_builder =
            taiko_reth_payload_builder::TaikoPayloadBuilder::new(evm_config, chain_spec);
        let conf = ctx.payload_builder_config();

        let payload_job_config = BasicPayloadJobGeneratorConfig::default()
            .interval(conf.interval())
            .deadline(conf.deadline())
            .max_payload_tasks(conf.max_payload_tasks())
            .extradata(conf.extradata_bytes());

        let payload_generator = BasicPayloadJobGenerator::with_builder(
            ctx.provider().clone(),
            pool,
            ctx.task_executor().clone(),
            payload_job_config,
            payload_builder,
        );
        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor().spawn_critical("payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

/// A basic ethereum payload service.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoNetworkBuilder;

impl<Node, Pool> NetworkBuilder<Node, Pool> for TaikoNetworkBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
    Pool: TransactionPool + Unpin + 'static,
{
    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<NetworkHandle> {
        let network = ctx.network_builder().await?;
        let handle = ctx.start_network(network, pool);

        Ok(handle)
    }
}

/// A basic ethereum consensus builder.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoConsensusBuilder;

impl<Node> ConsensusBuilder<Node> for TaikoConsensusBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
{
    type Consensus = Arc<dyn reth_consensus::Consensus>;

    async fn build_consensus(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Consensus> {
        let chain_spec = ctx.chain_spec();
        if ctx.is_dev() {
            Ok(Arc::new(AutoSealConsensus::new(chain_spec)))
        } else {
            Ok(Arc::new(TaikoBeaconConsensus::new(chain_spec)))
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoEngineValidatorBuilder;

impl<Node, Types> EngineValidatorBuilder<Node> for TaikoEngineValidatorBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ChainSpec>,
    Node: FullNodeTypes<Types = Types>,
    TaikoEngineValidator: EngineValidator<Types::Engine>,
{
    type Validator = TaikoEngineValidator;

    async fn build_validator(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Validator> {
        Ok(TaikoEngineValidator::new(ctx.chain_spec()))
    }
}
