use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_eth::{
    Block as RpcBlock, EIP1186AccountProofResponse, Header as RpcHeader, Transaction,
};
use async_trait::async_trait;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use reth_errors::RethError;
use reth_evm::execute::BlockExecutorProvider;
use reth_node_api::NodePrimitives;
use reth_primitives::{Block, TransactionSigned};
use reth_provider::{
    BlockNumReader, BlockReaderIdExt, ChainSpecProvider, EvmEnvProvider, L1OriginReader,
    StateProviderFactory,
};
use reth_rpc_eth_api::TransactionCompat;
use reth_rpc_server_types::ToRpcResult;
use reth_rpc_types_compat::transaction::{from_recovered, from_recovered_with_block_context};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_primitives::L1Origin;
use reth_taiko_proposer_consensus::{ProposerBuilder, ProposerClient};
use reth_tasks::TaskSpawner;
use reth_transaction_pool::{PoolConsensusTx, PoolTransaction, TransactionPool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Taiko rpc interface.
#[rpc(server, client, namespace = "taiko")]
pub trait TaikoApi {
    /// HeadL1Origin returns the latest L2 block's corresponding L1 origin.
    #[method(name = "headL1Origin")]
    async fn head_l1_origin(&self) -> RpcResult<L1Origin>;

    /// L1OriginByID returns the L2 block's corresponding L1 origin.
    #[method(name = "l1OriginByID")]
    async fn l1_origin_by_id(&self, block_id: BlockId) -> RpcResult<L1Origin>;

    /// GetSyncMode returns the node sync mode.
    #[method(name = "getSyncMode")]
    async fn get_sync_mode(&self) -> RpcResult<String> {
        Ok("full".to_string())
    }
}

/// Taiko rpc interface.
#[rpc(server, client, namespace = "taikoAuth")]
pub trait TaikoAuthApi {
    /// Get the transaction pool content.
    #[method(name = "txPoolContent")]
    async fn tx_pool_content(
        &self,
        beneficiary: Address,
        base_fee: u64,
        block_max_gas_limit: u64,
        max_bytes_per_tx_list: u64,
        local_accounts: Option<Vec<Address>>,
        max_transactions_lists: u64,
    ) -> RpcResult<Vec<PreBuiltTxList>>;

    /// Get the transaction pool content with the minimum tip.
    #[method(name = "txPoolContentWithMinTip")]
    async fn tx_pool_content_with_min_tip(
        &self,
        beneficiary: Address,
        base_fee: u64,
        block_max_gas_limit: u64,
        max_bytes_per_tx_list: u64,
        local_accounts: Option<Vec<Address>>,
        max_transactions_lists: u64,
        min_tip: u64,
    ) -> RpcResult<Vec<PreBuiltTxList>>;

    /// GetSyncMode returns the node sync mode.
    #[method(name = "provingPreFlight")]
    async fn proving_pre_flight(&self, block_id: BlockId) -> RpcResult<ProvingPreFlight> {
        todo!()
    }
}

/// `PreFlight` is the pre-flight data for the proving process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvingPreFlight {
    /// The block to be proven.
    pub block: RpcBlock,
    /// The parent header.
    pub parent_header: RpcHeader,
    /// The account proofs.
    pub account_proofs: Vec<EIP1186AccountProofResponse>,
    /// The parent account proofs.
    pub parent_account_proofs: Vec<EIP1186AccountProofResponse>,
    /// The contracts used.
    pub contracts: HashMap<B256, Bytes>,
    /// The ancestor used.
    pub ancestor_headers: Vec<RpcHeader>,
}

/// `PreBuiltTxList` is a pre-built transaction list based on the latest chain state,
/// with estimated gas used / bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PreBuiltTxList {
    /// The list of transactions.
    pub tx_list: Vec<Transaction>,
    /// The estimated gas used.
    pub estimated_gas_used: u64,
    /// The estimated bytes length.
    pub bytes_length: u64,
}

/// Taiko API.
#[derive(Debug)]
pub struct TaikoAuthApi<Provider, Pool, BlockExecutor, Eth> {
    proposer_client: ProposerClient,
    tx_resp_builder: Eth,
    _marker: std::marker::PhantomData<(Provider, Pool, BlockExecutor)>,
}

impl<Provider, Pool, BlockExecutor, Eth> TaikoAuthApi<Provider, Pool, BlockExecutor, Eth>
where
    Provider: StateProviderFactory
        + BlockReaderIdExt<Header = reth_primitives::Header>
        + ChainSpecProvider<ChainSpec = TaikoChainSpec>
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
    BlockExecutor: BlockExecutorProvider,
    BlockExecutor::Primitives: NodePrimitives<Block = Block>,
{
    /// Creates a new instance of `Taiko`.
    pub fn new(
        provider: Provider,
        pool: Pool,
        block_executor: BlockExecutor,
        task_spawner: Box<dyn TaskSpawner>,
        tx_resp_builder: Eth,
    ) -> Self {
        let chain_spec = provider.chain_spec();
        let (_, proposer_client, proposer_task) =
            ProposerBuilder::new(chain_spec, provider, pool, block_executor).build();
        task_spawner.spawn(Box::pin(proposer_task));

        Self { proposer_client, tx_resp_builder, _marker: Default::default() }
    }
}
/// Taiko API
#[derive(Debug)]
pub struct TaikoApi<Provider> {
    provider: Provider,
}

impl<Provider> TaikoApi<Provider> {
    /// Creates a new instance of `Taiko`.
    pub const fn new(provider: Provider) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl<Provider> TaikoApiServer for TaikoApi<Provider>
where
    Provider: L1OriginReader + 'static,
{
    /// HeadL1Origin returns the latest L2 block's corresponding L1 origin.
    async fn head_l1_origin(&self) -> RpcResult<L1Origin> {
        let res = self.provider.get_head_l1_origin().to_rpc_result();
        debug!(target: "rpc::taiko", ?res, "Read head l1 origin");
        res
    }

    /// L1OriginByID returns the L2 block's corresponding L1 origin.
    async fn l1_origin_by_id(&self, block_id: U256) -> RpcResult<L1Origin> {
        let block_number = block_id.try_into().map_err(RethError::other).to_rpc_result()?;
        let res = self.provider.get_l1_origin(block_number).to_rpc_result();
        debug!(target: "rpc::taiko", ?block_number, ?res, "Read l1 origin by id");
        res
    }
}

#[async_trait]
impl<Provider, Pool, BlockExecutor, Eth> TaikoAuthApiServer
    for TaikoAuthApi<Provider, Pool, BlockExecutor, Eth>
where
    Provider: StateProviderFactory + ChainSpecProvider + EvmEnvProvider + BlockNumReader + 'static,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>> + 'static,
    BlockExecutor: Send + Sync + 'static,
    Eth: TransactionCompat<PoolConsensusTx<Pool>, Transaction = Transaction> + 'static,
{
    /// TxPoolContent retrieves the transaction pool content with the given upper limits.
    async fn tx_pool_content(
        &self,
        beneficiary: Address,
        base_fee: u64,
        block_max_gas_limit: u64,
        max_bytes_per_tx_list: u64,
        local_accounts: Option<Vec<Address>>,
        max_transactions_lists: u64,
    ) -> RpcResult<Vec<PreBuiltTxList>> {
        self.tx_pool_content_with_min_tip(
            beneficiary,
            base_fee,
            block_max_gas_limit,
            max_bytes_per_tx_list,
            local_accounts,
            max_transactions_lists,
            0,
        )
        .await
    }

    /// TxPoolContent retrieves the transaction pool content with the given upper limits.
    async fn tx_pool_content_with_min_tip(
        &self,
        beneficiary: Address,
        base_fee: u64,
        block_max_gas_limit: u64,
        max_bytes_per_tx_list: u64,
        local_accounts: Option<Vec<Address>>,
        max_transactions_lists: u64,
        min_tip: u64,
    ) -> RpcResult<Vec<PreBuiltTxList>> {
        debug!(
            target: "rpc::taiko",
            ?beneficiary,
            ?base_fee,
            ?block_max_gas_limit,
            ?max_bytes_per_tx_list,
            ?local_accounts,
            ?max_transactions_lists,
            ?min_tip,
            "Read tx pool context"
        );
        let res = self
            .proposer_client
            .tx_pool_content_with_min_tip(
                beneficiary,
                base_fee,
                block_max_gas_limit,
                max_bytes_per_tx_list,
                local_accounts,
                max_transactions_lists,
                min_tip,
            )
            .await
            .map(|tx_lists| {
                tx_lists
                    .into_iter()
                    .map(|tx_list| PreBuiltTxList {
                        tx_list: tx_list
                            .txs
                            .iter()
                            .filter_map(|tx| tx.clone().into_ecrecovered())
                            .filter_map(|tx| from_recovered(tx, &self.tx_resp_builder).ok())
                            .collect(),
                        estimated_gas_used: tx_list.estimated_gas_used,
                        bytes_length: tx_list.bytes_length,
                    })
                    .collect()
            })
            .to_rpc_result();
        debug!(target: "rpc::taiko", ?res, "Read tx pool context");
        res
    }
}
