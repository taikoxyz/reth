use alloy_consensus::{proofs, BlockHeader, Header, Transaction as _};
use alloy_eips::{
    calc_excess_blob_gas, eip4895::Withdrawals, eip7685::Requests, merge::BEACON_NONCE, BlockId,
};
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_eth::{
    Block as RpcBlock, BlockTransactionsKind, EIP1186AccountProofResponse, Header as RpcHeader,
    Transaction,
};
use async_trait::async_trait;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use reth_chainspec::EthereumHardforks;
use reth_errors::RethError;
use reth_evm::{
    execute::{BlockExecutorProvider, Executor},
    ConfigureEvmEnv,
};
use reth_node_api::NodePrimitives;
use reth_primitives::{
    Block, BlockBody, BlockExt as _, InvalidTransactionError, SealedBlockWithSenders,
    TransactionSigned,
};
use reth_provider::{
    BlockExecutionInput, BlockExecutionOutput, BlockIdReader, BlockNumReader, BlockReader,
    BlockReaderIdExt, ChainSpecProvider, HeaderProvider, L1OriginReader, ProviderBlock,
    ProviderHeader, ProviderTx, StateProofProvider, StateProviderFactory, TransactionsProvider,
};
use reth_revm::database::StateProviderDatabase;
use reth_rpc_eth_api::{
    helpers::{SpawnBlocking, TraceExt},
    EthApiTypes, FromEthApiError as _, FromEvmError as _, RpcNodeCore,
};
use reth_rpc_eth_types::EthApiError;
use reth_rpc_server_types::ToRpcResult;
use reth_rpc_types_compat::{block::from_block, transaction::from_recovered, TransactionCompat};
use reth_taiko_evm::encode_and_compress_tx_list;
use reth_taiko_primitives::L1Origin;
use reth_tasks::pool::BlockingTaskGuard;
use reth_transaction_pool::{
    error::InvalidPoolTransactionError, pool::BestTransactionsWithPrioritizedSenders,
    BestTransactions, BestTransactionsAttributes, PoolTransaction, TransactionPool,
};
use revm::{
    db::{BundleState, CacheDB, State},
    primitives::{EVMError, Env, InvalidTransaction, ResultAndState},
    DatabaseCommit,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{AcquireError, OwnedSemaphorePermit};
use tracing::debug;

/// Taiko rpc interface.
#[cfg_attr(not(feature = "client"), rpc(server, namespace = "taiko"))]
#[cfg_attr(feature = "client", rpc(server, client, namespace = "taiko"))]
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
#[cfg_attr(not(feature = "client"), rpc(server, namespace = "taikoAuth"))]
#[cfg_attr(feature = "client", rpc(server, client, namespace = "taikoAuth"))]
pub trait TaikoAuthApi<T, H> {
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
    ) -> RpcResult<Vec<PreBuiltTxList<T>>>;

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
    ) -> RpcResult<Vec<PreBuiltTxList<T>>>;

    /// GetSyncMode returns the node sync mode.
    #[method(name = "provingPreflight")]
    async fn proving_preflight(&self, block_id: BlockId) -> RpcResult<ProvingPreflight<T, H>>;
}

/// `PreFlight` is the pre-flight data for the proving process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvingPreflight<T, H> {
    /// The block to be proven.
    pub block: RpcBlock<T, RpcHeader<H>>,
    /// The parent header.
    pub parent_header: RpcHeader<H>,
    /// The account proofs.
    pub account_proofs: Vec<EIP1186AccountProofResponse>,
    /// The parent account proofs.
    pub parent_account_proofs: Vec<EIP1186AccountProofResponse>,
    /// The contracts used.
    pub contracts: HashMap<B256, Bytes>,
    /// The ancestor used.
    pub ancestor_headers: Vec<RpcHeader<H>>,
}

/// `PreBuiltTxList` is a pre-built transaction list based on the latest chain state,
/// with estimated gas used / bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PreBuiltTxList<T> {
    /// The list of transactions.
    pub tx_list: Vec<T>,
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
    Eth: EthApiTypes + TraceExt + 'static,
{
    /// HeadL1Origin returns the latest L2 block's corresponding L1 origin.
    async fn head_l1_origin(&self) -> RpcResult<L1Origin> {
        let res = self
            .eth_api()
            .spawn_blocking_io(move |this| {
                this.provider().get_head_l1_origin().map_err(Eth::Error::from_eth_err)
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
                this.provider().get_l1_origin(block_number).map_err(Eth::Error::from_eth_err)
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

    /// Access the underlying `BlockExecutor`.
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

type TransactionCompatTx<P> = <P as Network>::TransactionResponse;

impl<Eth, BlockExecutor> TaikoAuthApi<Eth, BlockExecutor>
where
    Eth: EthApiTypes + TraceExt + 'static,
    BlockExecutor:
        BlockExecutorProvider<Primitives: NodePrimitives<Block = ProviderBlock<Eth::Provider>>>,
{
    /// Acquires a permit to execute a tracing call.
    async fn acquire_trace_permit(&self) -> Result<OwnedSemaphorePermit, AcquireError> {
        self.inner.blocking_task_guard.clone().acquire_owned().await
    }

    fn get_block_info(
        &self,
        block_number: u64,
    ) -> Result<(SealedBlockWithSenders<ProviderBlock<Eth::Provider>>, U256), Eth::Error> {
        let block = self
            .provider()
            .sealed_block_with_senders(block_number.into(), Default::default())
            .map_err(Eth::Error::from_eth_err)?
            .ok_or_else(|| EthApiError::HeaderNotFound(block_number.into()))?;
        let mut total_difficulty = self
            .provider()
            .header_td_by_number(block.number())
            .map_err(Eth::Error::from_eth_err)?;
        if total_difficulty.is_none() {
            // if we failed to find td after we successfully loaded the block, try again using
            // the hash this only matters if the chain is currently transitioning the merge block and there's a reorg: <https://github.com/paradigmxyz/reth/issues/10941>
            total_difficulty =
                self.provider().header_td(&block.hash()).map_err(Eth::Error::from_eth_err)?;
        }
        Ok((block, total_difficulty.unwrap_or_default()))
    }

    async fn preflight(
        &self,
        block_id: BlockId,
    ) -> Result<
        ProvingPreflight<TransactionCompatTx<Eth::NetworkTypes>, ProviderHeader<Eth::Provider>>,
        Eth::Error,
    > {
        let block_number = self
            .provider()
            .block_number_for_id(block_id)
            .map_err(Eth::Error::from_eth_err)?
            .ok_or_else(|| EthApiError::HeaderNotFound(block_id))?;
        if block_number == 0 {
            return Err(EthApiError::HeaderNotFound(block_id).into());
        }
        let parent_block_number = block_number - 1;

        let this = self.clone();
        self.eth_api()
            .spawn_with_state_at_block(parent_block_number.into(), move |parent_state| {
                let mut db =  CacheDB::new(StateProviderDatabase::new(parent_state));
                let (block, total_difficulty) = this.get_block_info(block_number)?;
                let block_hash = block.hash();
                let parent_hash = block.parent_hash();
                let (parent_block, parent_total_difficulty) = this.get_block_info(parent_block_number)?;

                debug!(target: "taiko::api", transactions = ?&block.body, "before executing transactions");
                // execute the block
                let block = block.unseal();
                let parent_block = parent_block.unseal();
                let BlockExecutionOutput { state, .. } =
                    this.block_executor().executor(&mut db).execute((&block, total_difficulty).into()).map_err(|err|EthApiError::Internal(err.into()))?;
                let rpc_block = from_block(block,total_difficulty,BlockTransactionsKind::Full, Some(block_hash), this.eth_api().tx_resp_builder())?;
                let rpc_parent_block =  from_block(parent_block, parent_total_difficulty, BlockTransactionsKind::Full, Some(parent_hash), this.eth_api().tx_resp_builder())?;

                let BundleState { state: bundle_state, contracts,.. } = state;

                let state = this.eth_api().state_at_block_id(block_hash.into())?;
                let parent_state = db.db.into_inner();

                let mut  ancestor_headers = vec![];

                for (touched_block_number, touched_block_hash) in db.block_hashes {
                    let (touched_block, touched_total_difficulty) =  this.get_block_info(touched_block_number.try_into().unwrap())?;
                    let rpc_touched_block = from_block(touched_block.unseal(), touched_total_difficulty, BlockTransactionsKind::Full, Some(touched_block_hash), this.eth_api().tx_resp_builder())?;
                    ancestor_headers.push(rpc_touched_block.header);
                }


                let mut account_proofs = vec![];
                let mut parent_account_proofs = vec![];

                for (address, account) in bundle_state {
                    let storage_keys: Vec<B256> = account.storage.into_keys().map(|key| key.into()).collect::<Vec<_>>();
                    let keys = storage_keys.clone().into_iter().map(|key| key.into()).collect::<Vec<_>>();

                    let parent_proof = parent_state
                        .proof(Default::default(), address, storage_keys.as_slice())
                        .map_err(Eth::Error::from_eth_err)?;
                    parent_account_proofs.push(parent_proof.into_eip1186_response(keys.clone()));

                    let proof = state
                        .proof(Default::default(), address, storage_keys.as_slice())
                        .map_err(Eth::Error::from_eth_err)?;
                    account_proofs.push(proof.into_eip1186_response(keys));
                }


                Ok(ProvingPreflight{
                    block: rpc_block,
                    parent_header: rpc_parent_block.header,
                    account_proofs,
                    parent_account_proofs,
                    contracts: contracts.into_iter().map(|(k, v)| (k, v.original_bytes())).collect(),
                    ancestor_headers,
                })

            })
            .await
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
    ) -> Result<Vec<PreBuiltTxList<TransactionCompatTx<Eth::NetworkTypes>>>, Eth::Error> {
        let last_num = self.provider().last_block_number().map_err(Eth::Error::from_eth_err)?;
        let ((cfg, block_env, _), block) = futures::try_join!(
            self.eth_api().evm_env_at(last_num.into()),
            self.eth_api().block_with_senders(last_num.into()),
        )?;
        // replay all transactions of the block
        self.eth_api()
            .spawn_tracing(move |this| {
                let best_txs = this
                    .pool()
                    .best_transactions_with_attributes(BestTransactionsAttributes::new(
                        base_fee, None,
                    ))
                    .filter_transactions(|tx| {
                        tx.effective_tip_per_gas(base_fee).is_some_and(|tip| tip >= min_tip as u128)
                    });
                let mut best_txs = BestTransactionsWithPrioritizedSenders::new(
                    local_accounts.unwrap_or_default().into_iter().collect(),
                    u64::MAX,
                    best_txs,
                );
                best_txs.skip_blobs();

                let mut db = State::builder()
                    .with_database(StateProviderDatabase::new(
                        this.provider().latest().map_err(Eth::Error::from_eth_err)?,
                    ))
                    .with_bundle_update()
                    .build();

                let mut target_list: Vec<PreBuiltTxList<_>> = vec![];

                for _ in 0..max_transactions_lists {
                    let mut cumulative_gas_used = 0;
                    let mut tx_list = vec![];
                    let mut buf_len: u64 = 0;

                    while let Some(pool_tx) = best_txs.next() {
                        let tx = pool_tx.to_consensus();

                        // Configure the environment for the block.
                        let env = Env::boxed(
                            cfg.cfg_env.clone(),
                            block_env.clone(),
                            this.evm_config().tx_env(tx.as_signed(), tx.signer(), None),
                        );

                        let mut evm = revm::Evm::builder().with_env(env).with_db(&mut db).build();

                        let ResultAndState { result, state } = match evm.transact() {
                            Ok(res) => res,
                            Err(err) => {
                                match err {
                                    EVMError::Transaction(err) => {
                                        if matches!(err, InvalidTransaction::NonceTooLow { .. }) {
                                            // if the nonce is too low, we can skip this transaction
                                        } else {
                                            // if the transaction is invalid, we can skip it and all
                                            // of its
                                            // descendants
                                            best_txs.mark_invalid(
                                                &pool_tx,
                                                InvalidPoolTransactionError::Consensus(
                                                    InvalidTransactionError::TxTypeNotSupported,
                                                ),
                                            );
                                        }
                                        continue
                                    }
                                    err => {
                                        // this is an error that we should treat as fatal for this
                                        // attempt
                                        return Err(Eth::Error::from_evm_err(err))
                                    }
                                }
                            }
                        };
                        tx_list.push(tx.clone());
                        // commit changes

                        let compressed_buf = encode_and_compress_tx_list(&tx_list)
                            .map_err(|err| EthApiError::EvmCustom(err.to_string()))?;
                        if compressed_buf.len() > max_bytes_per_tx_list as usize {
                            tx_list.pop();
                            break;
                        }

                        buf_len = compressed_buf.len() as u64;
                        // append gas used
                        cumulative_gas_used += result.gas_used();
                        drop(evm);
                        db.commit(state);
                    }

                    if tx_list.is_empty() {
                        break;
                    }
                    target_list.push(PreBuiltTxList {
                        tx_list: tx_list
                            .into_iter()
                            .filter_map(|tx| from_recovered(tx, this.tx_resp_builder()).ok())
                            .collect(),
                        estimated_gas_used: cumulative_gas_used,
                        bytes_length: buf_len,
                    });
                }

                Ok(target_list)
            })
            .await
    }
}

#[async_trait]
impl<Eth, BlockExecutor>
    TaikoAuthApiServer<TransactionCompatTx<Eth::NetworkTypes>, ProviderHeader<Eth::Provider>>
    for TaikoAuthApi<Eth, BlockExecutor>
where
    Eth: EthApiTypes + TraceExt + 'static,
    BlockExecutor:
        BlockExecutorProvider<Primitives: NodePrimitives<Block = ProviderBlock<Eth::Provider>>>,
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
    ) -> RpcResult<Vec<PreBuiltTxList<TransactionCompatTx<Eth::NetworkTypes>>>> {
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
    ) -> RpcResult<Vec<PreBuiltTxList<TransactionCompatTx<Eth::NetworkTypes>>>> {
        let _permit = self
            .acquire_trace_permit()
            .await
            .map_err(RethError::other)
            .map_err(EthApiError::Internal)?;
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
            .map_err(Into::into);
        debug!(target: "rpc::taiko", ?res, "Read tx pool context");
        res
    }

    async fn proving_preflight(
        &self,
        block_id: BlockId,
    ) -> RpcResult<
        ProvingPreflight<TransactionCompatTx<Eth::NetworkTypes>, ProviderHeader<Eth::Provider>>,
    > {
        let _permit = self
            .acquire_trace_permit()
            .await
            .map_err(RethError::other)
            .map_err(EthApiError::Internal)?;
        debug!(target: "rpc::taiko", ?block_id, "Read proving preflight");
        let res = self.preflight(block_id).await.map_err(Into::into);
        debug!(target: "rpc::taiko", ?res, "Read proving pre flight");
        res
    }
}
