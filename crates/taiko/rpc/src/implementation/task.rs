use futures_util::{future::BoxFuture, FutureExt};
use reth_evm::execute::BlockExecutorProvider;
use reth_primitives::{Block, NodePrimitives, TransactionSigned};
use reth_provider::{BlockReaderIdExt, StateProviderFactory};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_transaction_pool::{PoolTransaction, TransactionPool, ValidPoolTransaction};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::debug;

use super::{proposer::build_and_execute, TaikoImplMessage};

/// A Future that listens for new ready transactions and puts new blocks into storage
pub struct TaikoImplTask<Provider, Pool: TransactionPool, Executor> {
    /// The configured chain spec
    chain_spec: Arc<TaikoChainSpec>,
    /// The client used to interact with the state
    provider: Provider,
    /// Single active future that inserts a new block into `storage`
    insert_task: Option<BoxFuture<'static, ()>>,
    /// Pool where transactions are stored
    pool: Pool,
    /// backlog of sets of transactions ready to be mined
    #[allow(clippy::type_complexity)]
    queued: VecDeque<(
        TaikoImplMessage,
        Vec<Arc<ValidPoolTransaction<<Pool as TransactionPool>::Transaction>>>,
    )>,
    /// The type used for block execution
    block_executor: Executor,
    rx: UnboundedReceiver<TaikoImplMessage>,
}

// === impl MiningTask ===

impl<Executor, Provider, Pool: TransactionPool> TaikoImplTask<Provider, Pool, Executor> {
    /// Creates a new instance of the task
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        chain_spec: Arc<TaikoChainSpec>,
        provider: Provider,
        pool: Pool,
        block_executor: Executor,
        rx: UnboundedReceiver<TaikoImplMessage>,
    ) -> Self {
        Self {
            chain_spec,
            provider,
            insert_task: None,
            pool,
            queued: Default::default(),
            block_executor,
            rx,
        }
    }
}

impl<Executor, Provider, Pool> Future for TaikoImplTask<Provider, Pool, Executor>
where
    Provider: StateProviderFactory
        + BlockReaderIdExt<Header = reth_primitives::Header>
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
    Executor: BlockExecutorProvider,
    Executor::Primitives: NodePrimitives<Block = Block>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // this drives block production and
        loop {
            if let Some(msg) = match this.rx.poll_recv(cx) {
                Poll::Ready(Some(msg)) => Some(msg),
                Poll::Ready(None) => return Poll::Ready(()),
                _ => None,
            } {
                match &msg {
                    TaikoImplMessage::PoolContent {
                        base_fee, local_accounts, min_tip, tx, ..
                    } => {
                        let mut best_txs = this.pool.best_transactions();
                        best_txs.skip_blobs();
                        debug!(target: "taiko::proposer", txs = ?best_txs.size_hint(), "Proposer get best transactions");
                        let (mut local_txs, remote_txs): (Vec<_>, Vec<_>) = best_txs
                            .filter(|tx| {
                                tx.effective_tip_per_gas(*base_fee)
                                    .is_some_and(|tip| tip >= *min_tip as u128)
                            })
                            .partition(|tx| {
                                local_accounts.as_ref().is_some_and(|local_accounts| {
                                    local_accounts.contains(&tx.sender())
                                })
                            });
                        local_txs.extend(remote_txs);
                        debug!(target: "taiko::proposer", txs = ?local_txs.len(), "Proposer filter best transactions");

                        // miner returned a set of transaction that we feed to the producer
                        this.queued.push_back((msg, local_txs));
                    }
                    TaikoImplMessage::ProvingPreFlight { .. } => {
                        this.queued.push_back((msg, vec![]));
                    }
                }
            };

            if this.insert_task.is_none() {
                if this.queued.is_empty() {
                    // nothing to insert
                    break;
                }

                // ready to queue in new insert task;
                let (msg, txs) = this.queued.pop_front().expect("not empty");

                let client = this.provider.clone();
                let chain_spec = Arc::clone(&this.chain_spec);
                let executor = this.block_executor.clone();

                // Create the mining future that creates a block, notifies the engine that drives
                // the pipeline
                this.insert_task = Some(Box::pin(async move {
                    match msg {
                        TaikoImplMessage::PoolContent {
                            beneficiary,
                            base_fee,
                            block_max_gas_limit,
                            max_bytes_per_tx_list,
                            max_transactions_lists,
                            tx,
                            ..
                        } => {
                            let txs: Vec<_> =
                                txs.into_iter().map(|tx| tx.to_consensus().into_signed()).collect();
                            let ommers = vec![];
                            let res = build_and_execute(
                                txs,
                                ommers,
                                &client,
                                chain_spec,
                                &executor,
                                beneficiary,
                                block_max_gas_limit,
                                max_bytes_per_tx_list,
                                max_transactions_lists,
                                base_fee,
                            );
                            let _ = tx.send(res);
                        }
                        TaikoImplMessage::ProvingPreFlight { block_id, tx } => todo!(),
                    }
                }));
            }

            if let Some(mut fut) = this.insert_task.take() {
                match fut.poll_unpin(cx) {
                    Poll::Ready(_) => {}
                    Poll::Pending => {
                        this.insert_task = Some(fut);
                        break;
                    }
                }
            }
        }

        Poll::Pending
    }
}

impl<Client, Pool: TransactionPool, EvmConfig: std::fmt::Debug> std::fmt::Debug
    for TaikoImplTask<Client, Pool, EvmConfig>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiningTask").finish_non_exhaustive()
    }
}
