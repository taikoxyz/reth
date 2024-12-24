use alloy_consensus::{proofs, BlockHeader, Header, Transaction as _};
use alloy_eips::{
    calc_excess_blob_gas, eip4895::Withdrawals, eip7685::Requests, merge::BEACON_NONCE, BlockId,
};
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_eth::{
    Block as RpcBlock, EIP1186AccountProofResponse, Header as RpcHeader, Transaction,
};
use async_trait::async_trait;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use reth_chainspec::EthereumHardforks;
use reth_errors::{BlockExecutionError, BlockValidationError, ProviderError, RethError};
use reth_evm::execute::{BlockExecutorProvider, Executor};
use reth_node_api::NodePrimitives;
use reth_primitives::{Block, BlockBody, BlockExt as _, SealedBlockWithSenders, TransactionSigned};
use reth_primitives_traits::{Block as _, BlockBody as _, SignedTransaction};
use reth_provider::{
    BlockExecutionInput, BlockExecutionOutput, BlockIdReader, BlockNumReader, BlockReaderIdExt,
    ChainSpecProvider, EvmEnvProvider, L1OriginReader, ProviderBlock, StateProviderFactory,
    TaskResult, TransactionsProvider,
};
use reth_revm::database::StateProviderDatabase;
use reth_rpc_eth_api::{
    helpers::{SpawnBlocking, TraceExt},
    EthApiTypes, FromEthApiError, RpcNodeCore, TransactionCompat,
};
use reth_rpc_eth_types::EthApiError;
use reth_rpc_server_types::ToRpcResult;
use reth_rpc_types_compat::transaction::from_recovered;
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_primitives::L1Origin;
use reth_tasks::{pool::BlockingTaskGuard, TaskSpawner};
use reth_transaction_pool::{
    BestTransactionsAttributes, PoolConsensusTx, PoolTransaction, TransactionPool,
};
use revm::db::CacheDB;
use revm_primitives::{BlockEnv, CfgEnvWithHandlerCfg};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{AcquireError, OwnedSemaphorePermit};
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
    #[method(name = "provingPreflight")]
    async fn proving_preflight(&self, block_id: BlockId) -> RpcResult<ProvingPreflight>;
}

/// `PreFlight` is the pre-flight data for the proving process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvingPreflight {
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

/// Taiko API
#[derive(Debug)]
pub struct TaikoApi<Eth> {
    inner: Arc<TaikoApiInner<Eth>>,
}

#[derive(Debug)]
struct TaikoApiInner<Eth> {
    /// The implementation of `eth` API
    eth_api: Eth,
}

impl<Eth> TaikoApi<Eth> {
    /// Create a new instance of the [`DebugApi`]
    pub fn new(eth: Eth) -> Self {
        let inner = Arc::new(TaikoApiInner { eth_api: eth });
        Self { inner }
    }

    /// Access the underlying `Eth` API.
    pub fn eth_api(&self) -> &Eth {
        &self.inner.eth_api
    }
}

#[async_trait]
impl<Eth> TaikoApiServer for TaikoApi<Eth>
where
    Eth: EthApiTypes + SpawnBlocking + L1OriginReader + 'static,
{
    /// HeadL1Origin returns the latest L2 block's corresponding L1 origin.
    async fn head_l1_origin(&self) -> RpcResult<L1Origin> {
        let res = self
            .eth_api()
            .spawn_blocking_io(move |this| {
                this.get_head_l1_origin().map_err(Eth::Error::from_eth_err)
            })
            .await
            .map_err(Into::into);
        debug!(target: "rpc::taiko", ?res, "Read head l1 origin");
        res
    }

    /// L1OriginByID returns the L2 block's corresponding L1 origin.
    async fn l1_origin_by_id(&self, block_id: BlockId) -> RpcResult<L1Origin> {
        let block_number =
            block_id.as_u64().ok_or_else(|| RethError::msg("invalid block id")).to_rpc_result()?;
        let res = self
            .eth_api()
            .spawn_blocking_io(move |this| {
                this.get_l1_origin(block_number).map_err(Eth::Error::from_eth_err)
            })
            .await
            .map_err(Into::into);
        debug!(target: "rpc::taiko", ?block_number, ?res, "Read l1 origin by id");
        res
    }
}

/// Taiko API.
#[derive(Debug)]
pub struct TaikoAuthApi<Eth, BlockExecutor> {
    inner: Arc<TaikoAuthApiInner<Eth, BlockExecutor>>,
}

impl<Eth, BlockExecutor> TaikoAuthApi<Eth, BlockExecutor> {
    /// Create a new instance of the [`DebugApi`]
    pub fn new(
        eth: Eth,
        blocking_task_guard: BlockingTaskGuard,
        block_executor: BlockExecutor,
    ) -> Self {
        let inner =
            Arc::new(TaikoAuthApiInner { eth_api: eth, blocking_task_guard, block_executor });
        Self { inner }
    }

    /// Access the underlying `Eth` API.
    pub fn eth_api(&self) -> &Eth {
        &self.inner.eth_api
    }

    pub fn block_executor(&self) -> &BlockExecutor {
        &self.inner.block_executor
    }
}

impl<Eth: RpcNodeCore, BlockExecutor> TaikoAuthApi<Eth, BlockExecutor> {
    /// Access the underlying provider.
    pub fn provider(&self) -> &Eth::Provider {
        self.inner.eth_api.provider()
    }
}

#[derive(Debug)]
struct TaikoAuthApiInner<Eth, BlockExecutor> {
    /// The implementation of `eth` API
    eth_api: Eth,
    // restrict the number of concurrent calls to blocking calls
    blocking_task_guard: BlockingTaskGuard,
    /// block executor for debug & trace apis
    block_executor: BlockExecutor,
}

impl<Eth, BlockExecutor> Clone for TaikoAuthApi<Eth, BlockExecutor> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<Eth, BlockExecutor> TaikoAuthApi<Eth, BlockExecutor>
where
    Eth: EthApiTypes + TraceExt + 'static,
    <Eth as RpcNodeCore>::Provider: TransactionsProvider<Transaction = TransactionSigned> + 'static,
    BlockExecutor: BlockExecutorProvider<Primitives: NodePrimitives<Block = Block>>,
{
    /// Acquires a permit to execute a tracing call.
    async fn acquire_trace_permit(&self) -> Result<OwnedSemaphorePermit, AcquireError> {
        self.inner.blocking_task_guard.clone().acquire_owned().await
    }

    async fn pool_content(
        &self,
        beneficiary: Address,
        base_fee: u64,
        block_max_gas_limit: u64,
        max_bytes_per_tx_list: u64,
        local_accounts: Option<Vec<Address>>,
        max_transactions_lists: u64,
        min_tip: u64,
    ) -> Result<Vec<TaskResult>, Eth::Error> {
        let that = self.clone();
        let last_num = self.provider().last_block_number().map_err(Eth::Error::from_eth_err)?;
        // replay all transactions of the block
        self.eth_api().spawn_tracing(move |this| {
            let mut best_txs = this.pool().best_transactions_with_attributes(
                BestTransactionsAttributes::new(base_fee, None),
            );
            best_txs.skip_blobs();
            debug!(target: "taiko::proposer", txs = ?best_txs.size_hint(), "Proposer get best transactions");
            let (mut local_txs, remote_txs): (Vec<_>, Vec<_>) = best_txs
                .filter(|tx| {
                    tx.effective_tip_per_gas(base_fee)
                        .is_some_and(|tip| tip >= min_tip as u128)
                })
                .partition(|tx| {
                    local_accounts.as_ref().is_some_and(|local_accounts| {
                        local_accounts.contains(&tx.sender())
                    })
                });
            local_txs.extend(remote_txs);
            let txs: Vec<_> = local_txs.into_iter().map(|tx| tx.to_consensus().into_signed()).collect();

            let ommers = vec![];

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

            let chain_spec = that.provider().chain_spec();

            // if shanghai is active, include empty withdrawals
            let withdrawals =
                chain_spec.is_shanghai_active_at_timestamp(timestamp).then_some(Withdrawals::default());
            // if prague is active, include empty requests
            let requests =
                chain_spec.is_prague_active_at_timestamp(timestamp).then_some(Requests::default());

            let base_fee_per_gas = Some(base_fee);

            let blob_gas_used = chain_spec.is_cancun_active_at_timestamp(timestamp).then(|| {
                let mut sum_blob_gas_used = 0;
                for tx in &txs {
                    if let Some(tx_blob_gas) = tx.blob_gas_used() {
                        sum_blob_gas_used +=tx_blob_gas;
                    }
                }
                sum_blob_gas_used
            });
            let latest_block =
                this.provider().latest_header().map_err(Eth::Error::from_eth_err)?.ok_or_else(||EthApiError::HeaderNotFound(last_num.into()))?;
            let mut header = Header {
                parent_hash: latest_block.hash(),
                ommers_hash: proofs::calculate_ommers_root(&ommers),
                beneficiary,
                state_root: Default::default(),
                transactions_root: proofs::calculate_transaction_root(&txs),
                receipts_root: Default::default(),
                withdrawals_root: withdrawals.as_ref().map(|w| proofs::calculate_withdrawals_root(w)),
                logs_bloom: Default::default(),
                difficulty: U256::ZERO,
                number: latest_block.number() + 1,
                gas_limit: block_max_gas_limit,
                gas_used: 0,
                timestamp,
                mix_hash: Default::default(),
                nonce: BEACON_NONCE.into(),
                base_fee_per_gas,
                blob_gas_used,
                excess_blob_gas: None,
                extra_data: Default::default(),
                parent_beacon_block_root: None,
                requests_hash: requests.map(|r| r.requests_hash()),
                target_blobs_per_block: None,
            };

            if chain_spec.is_cancun_active_at_timestamp(timestamp) {
                header.parent_beacon_block_root = latest_block.parent_beacon_block_root();
                header.blob_gas_used = Some(0);

                let (parent_excess_blob_gas, parent_blob_gas_used) =
                    if chain_spec.is_cancun_active_at_timestamp(latest_block.timestamp()) {
                        (
                            latest_block.excess_blob_gas().unwrap_or_default(),
                            latest_block.blob_gas_used().unwrap_or_default(),
                        )
                    } else {
                        (0, 0)
                    };

                header.excess_blob_gas =
                    Some(calc_excess_blob_gas(parent_excess_blob_gas, parent_blob_gas_used))
            }

            let block = Block { header, body: BlockBody { transactions: txs, ommers, withdrawals } }
                .with_recovered_senders()
                .ok_or(EthApiError::Internal(BlockExecutionError::Validation(BlockValidationError::SenderRecoveryError).into()))?;

            debug!(target: "taiko::proposer", transactions=?&block.body, "before executing transactions");

            let mut db =
                StateProviderDatabase::new(this.provider().latest().map_err(Eth::Error::from_eth_err)?);

            // execute the block
            let block_input = BlockExecutionInput {
                block: &block,
                total_difficulty: U256::ZERO,
                enable_anchor: false,
                enable_skip: false,
                enable_build: true,
                max_bytes_per_tx_list,
                max_transactions_lists,
            };
            let BlockExecutionOutput { target_list, .. } =
                that.block_executor().executor(&mut db).execute(block_input).map_err(|err|EthApiError::Internal(err.into()))?;

            Ok(target_list)
        }).await
    }
}

#[async_trait]
impl<Eth, BlockExecutor> TaikoAuthApiServer for TaikoAuthApi<Eth, BlockExecutor>
where
    Eth: EthApiTypes + TraceExt + 'static,
    <Eth as EthApiTypes>::NetworkTypes: Network<TransactionResponse = Transaction>,
    <Eth as RpcNodeCore>::Provider: TransactionsProvider<Transaction = TransactionSigned> + 'static,
    BlockExecutor: BlockExecutorProvider<Primitives: NodePrimitives<Block = Block>>,
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
            .pool_content(
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
                            .filter_map(|tx| {
                                from_recovered(tx, self.eth_api().tx_resp_builder()).ok()
                            })
                            .collect(),
                        estimated_gas_used: tx_list.estimated_gas_used,
                        bytes_length: tx_list.bytes_length,
                    })
                    .collect()
            })
            .map_err(Into::into);
        debug!(target: "rpc::taiko", ?res, "Read tx pool context");
        res
    }

    async fn proving_preflight(&self, block_id: BlockId) -> RpcResult<ProvingPreflight> {
        // debug!(target: "rpc::taiko", ?block_id, "Read proving preflight");
        // let res = self.taiko_impl_client.proving_pre_flight(block_id).await.to_rpc_result();
        // debug!(target: "rpc::taiko", ?res, "Read proving pre flight");
        // res
        todo!()
    }
}
