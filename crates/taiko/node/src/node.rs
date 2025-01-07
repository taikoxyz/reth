//! Ethereum Node types config.

use crate::{TaikoEngineTypes, TaikoEngineValidator, TaikoEvmConfig};
use alloy_primitives::BlockNumber;
use reth_basic_payload_builder::{BasicPayloadJobGenerator, BasicPayloadJobGeneratorConfig};
use reth_evm::execute::BasicBlockExecutorProvider;
use reth_network::{NetworkHandle, PeersInfo};
use reth_node_builder::{
    components::{
        ComponentsBuilder, ConsensusBuilder, ExecutorBuilder, NetworkBuilder,
        PayloadServiceBuilder, PoolBuilder,
    },
    node::{FullNodeTypes, NodeTypes},
    rpc::{EngineValidatorBuilder, RpcAddOns},
    AddOnsContext, BuilderContext, FullNodeComponents, Node, NodeAdapter, NodeComponentsBuilder,
    NodeTypesWithDB, NodeTypesWithEngine, PayloadBuilderConfig, PayloadTypes, TxTy,
};
use reth_payload_builder::{EthBuiltPayload, PayloadBuilderHandle, PayloadBuilderService};
use reth_primitives::{EthPrimitives, PooledTransactionsElement};
use reth_provider::{CanonStateSubscriptions, EthStorage, L1OriginReader, ProviderResult};
use reth_rpc::EthApi;
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_consensus::TaikoBeaconConsensus;
use reth_taiko_engine_primitives::TaikoPayloadBuilderAttributes;
use reth_taiko_engine_types::TaikoPayloadAttributes;
use reth_taiko_evm::TaikoExecutionStrategyFactory;
use reth_taiko_primitives::L1Origin;
use reth_tracing::tracing::{debug, info};
use reth_transaction_pool::{
    blobstore::DiskFileBlobStore, EthTransactionPool, PoolTransaction, TransactionPool,
    TransactionValidationTaskExecutor,
};
use reth_trie_db::MerklePatriciaTrie;
use std::sync::Arc;

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
    >
    where
        Node:
            FullNodeTypes<Types: NodeTypes<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>>,
        <Node::Types as NodeTypesWithEngine>::Engine: PayloadTypes<
            BuiltPayload = EthBuiltPayload,
            PayloadAttributes = TaikoPayloadAttributes,
            PayloadBuilderAttributes = TaikoPayloadBuilderAttributes,
        >,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(TaikoPoolBuilder::default())
            .payload(TaikoPayloadBuilder::default())
            .network(TaikoNetworkBuilder::default())
            .executor(TaikoExecutorBuilder::default())
            .consensus(TaikoConsensusBuilder::default())
    }
}

impl NodeTypes for TaikoNode {
    type Primitives = EthPrimitives;
    type ChainSpec = TaikoChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
}

impl NodeTypesWithEngine for TaikoNode {
    type Engine = TaikoEngineTypes;
}

/// Add-ons w.r.t. l1 ethereum.
pub type TaikoAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    TaikoEngineValidatorBuilder,
>;

impl<Types, N> Node<N> for TaikoNode
where
    Types: NodeTypesWithDB
        + NodeTypesWithEngine<
            Engine = TaikoEngineTypes,
            ChainSpec = TaikoChainSpec,
            Primitives = EthPrimitives,
            Storage = EthStorage,
        >,
    N: FullNodeTypes<Types = Types>,
{
    type AddOns = TaikoAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    type ComponentsBuilder = ComponentsBuilder<
        N,
        TaikoPoolBuilder,
        TaikoPayloadBuilder,
        TaikoNetworkBuilder,
        TaikoExecutorBuilder,
        TaikoConsensusBuilder,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }

    fn add_ons(&self) -> Self::AddOns {
        TaikoAddOns::default()
    }
}

/// A regular ethereum evm and executor builder.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoExecutorBuilder;

impl<Types, Node> ExecutorBuilder<Node> for TaikoExecutorBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
{
    type EVM = TaikoEvmConfig;
    type Executor = BasicBlockExecutorProvider<TaikoExecutionStrategyFactory>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let evm_config = TaikoEvmConfig::new(ctx.chain_spec());
        let strategy_factory =
            TaikoExecutionStrategyFactory::new(ctx.chain_spec(), evm_config.clone());
        let executor = BasicBlockExecutorProvider::new(strategy_factory);

        Ok((evm_config, executor))
    }
}

/// A basic ethereum transaction pool.
///
/// This contains various settings that can be configured and take precedence over the node's
/// config.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TaikoPoolBuilder {
    // TODO add options for txpool args
}

impl<Types, Node> PoolBuilder<Node> for TaikoPoolBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
{
    type Pool = EthTransactionPool<Node::Provider, DiskFileBlobStore>;

    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let pool_config = ctx.pool_config();
        let blob_store = DiskFileBlobStore::open(data_dir.blobstore(), Default::default())?;
        let validator = TransactionValidationTaskExecutor::eth_builder(Arc::new(
            ctx.chain_spec().inner.clone(),
        ))
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

impl<Types, Node, Pool> PayloadServiceBuilder<Node, Pool> for TaikoPayloadBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TxTy<Node::Types>>>
        + Unpin
        + 'static,
    Types::Engine: PayloadTypes<
        BuiltPayload = EthBuiltPayload,
        PayloadAttributes = TaikoPayloadAttributes,
        PayloadBuilderAttributes = TaikoPayloadBuilderAttributes,
    >,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<Types::Engine>> {
        let chain_spec = ctx.chain_spec();
        let evm_config = TaikoEvmConfig::new(chain_spec.clone());
        let payload_builder =
            reth_taiko_payload_builder::TaikoPayloadBuilder::new(evm_config, chain_spec);
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
pub struct TaikoNetworkBuilder {
    // TODO add closure to modify network
}

impl<Node, Pool> NetworkBuilder<Node, Pool> for TaikoNetworkBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>>,
    Pool: TransactionPool<
            Transaction: PoolTransaction<
                Consensus = TxTy<Node::Types>,
                Pooled = PooledTransactionsElement,
            >,
        > + Unpin
        + 'static,
{
    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<NetworkHandle> {
        let network = ctx.network_builder().await?;
        let handle = ctx.start_network(network, pool);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

/// A basic ethereum consensus builder.
#[derive(Debug, Default, Clone, Copy)]
pub struct TaikoConsensusBuilder {
    // TODO add closure to modify consensus
}

impl<Node> ConsensusBuilder<Node> for TaikoConsensusBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = TaikoChainSpec, Primitives = EthPrimitives>>,
{
    type Consensus = Arc<dyn reth_consensus::FullConsensus>;

    async fn build_consensus(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Consensus> {
        Ok(Arc::new(TaikoBeaconConsensus::new(
            ctx.chain_spec(),
            ProviderWithDebug(ctx.provider().clone()),
        )))
    }
}

struct ProviderWithDebug<Provider>(Provider);

impl<Provider> std::fmt::Debug for ProviderWithDebug<Provider> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderWithDebug").finish()
    }
}

impl<Provider> L1OriginReader for ProviderWithDebug<Provider>
where
    Provider: L1OriginReader,
{
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin> {
        self.0.get_l1_origin(block_number)
    }

    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin> {
        self.0.get_head_l1_origin()
    }

    fn get_head_l1_origin_number(&self) -> ProviderResult<BlockNumber> {
        self.0.get_head_l1_origin_number()
    }
}

/// Builder for [`EthereumEngineValidator`].
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct TaikoEngineValidatorBuilder;

impl<Node, Types> EngineValidatorBuilder<Node> for TaikoEngineValidatorBuilder
where
    Types: NodeTypesWithEngine<
        ChainSpec = TaikoChainSpec,
        Engine = TaikoEngineTypes,
        Primitives = EthPrimitives,
    >,
    Node: FullNodeComponents<Types = Types>,
{
    type Validator = TaikoEngineValidator;

    async fn build(self, ctx: &AddOnsContext<'_, Node>) -> eyre::Result<Self::Validator> {
        Ok(TaikoEngineValidator::new(ctx.config.chain.clone()))
    }
}
