//! Ethereum block executor.

use crate::{
    dao_fork::{DAO_HARDFORK_BENEFICIARY, DAO_HARDKFORK_ACCOUNTS},
    TaikoEvmConfig,
};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use reth_chainspec::{ChainSpec, TAIKO_HEKLA, TAIKO_MAINNET};
use reth_consensus::ConsensusError;
use reth_evm::{
    execute::{
        BatchExecutor, BlockExecutionError, BlockExecutionInput, BlockExecutionOutput,
        BlockExecutorProvider, BlockValidationError, Executor, ProviderError,
    },
    ConfigureEvm,
};
use reth_execution_types::ExecutionOutcome;
use reth_primitives::{
    BlockNumber, BlockWithSenders, Hardfork, Header, Receipt, Request, TransactionSigned,
    Withdrawals, U256,
};
use reth_prune_types::PruneModes;
use reth_revm::{
    batch::{BlockBatchRecord, BlockExecutorStats},
    db::{states::bundle_state::BundleRetention, State},
    state_change::{
        apply_beacon_root_contract_call, apply_blockhashes_update,
        apply_withdrawal_requests_contract_call, post_block_balance_increments,
    },
    Evm, JournaledState,
};
use reth_taiko_beacon_consensus::{
    check_anchor_tx, decode_ontake_extra_data, validate_block_post_execution,
};
use revm_primitives::{
    db::{Database, DatabaseCommit},
    Address, BlockEnv, CfgEnvWithHandlerCfg, EnvWithHandlerCfg, HashSet, ResultAndState,
};

#[cfg(not(feature = "std"))]
use alloc::{sync::Arc, vec, vec::Vec};
use std::io;
use std::io::Write;
use tracing::debug;

use reth_evm::execute::TaskResult;
use reth_primitives::transaction::TransactionSignedList;
use reth_revm::interpreter::Host;
use revm_primitives::alloy_primitives::private::alloy_rlp;
#[cfg(feature = "std")]
use std::sync::Arc;

/// Provides executors to execute regular ethereum blocks
#[derive(Debug, Clone)]
pub struct TaikoExecutorProvider<EvmConfig = TaikoEvmConfig> {
    chain_spec: Arc<ChainSpec>,
    evm_config: EvmConfig,
}

impl TaikoExecutorProvider {
    /// Creates a new default ethereum executor provider.
    pub fn taiko(chain_spec: Arc<ChainSpec>) -> Self {
        Self::new(chain_spec, Default::default())
    }
}

impl<EvmConfig> TaikoExecutorProvider<EvmConfig> {
    /// Creates a new executor provider.
    pub const fn new(chain_spec: Arc<ChainSpec>, evm_config: EvmConfig) -> Self {
        Self { chain_spec, evm_config }
    }
}

impl<EvmConfig> TaikoExecutorProvider<EvmConfig>
where
    EvmConfig: ConfigureEvm,
{
    fn taiko_executor<DB>(&self, db: DB) -> TaikoBlockExecutor<EvmConfig, DB>
    where
        DB: Database<Error = ProviderError>,
    {
        TaikoBlockExecutor::new(
            self.chain_spec.clone(),
            self.evm_config.clone(),
            State::builder().with_database(db).with_bundle_update().without_state_clear().build(),
        )
    }
}

impl<EvmConfig> BlockExecutorProvider for TaikoExecutorProvider<EvmConfig>
where
    EvmConfig: ConfigureEvm,
{
    type Executor<DB: Database<Error = ProviderError>> = TaikoBlockExecutor<EvmConfig, DB>;

    type BatchExecutor<DB: Database<Error = ProviderError>> = TaikoBatchExecutor<EvmConfig, DB>;

    fn executor<DB>(&self, db: DB) -> Self::Executor<DB>
    where
        DB: Database<Error = ProviderError>,
    {
        self.taiko_executor(db)
    }

    fn batch_executor<DB>(&self, db: DB, prune_modes: PruneModes) -> Self::BatchExecutor<DB>
    where
        DB: Database<Error = ProviderError>,
    {
        let executor = self.taiko_executor(db);
        TaikoBatchExecutor {
            executor,
            batch_record: BlockBatchRecord::new(prune_modes),
            stats: BlockExecutorStats::default(),
        }
    }
}

/// Helper type for the output of executing a block.
#[derive(Debug, Clone)]
struct TaikoExecuteOutput {
    receipts: Vec<Receipt>,
    requests: Vec<Request>,
    gas_used: u64,
}

/// Helper container type for EVM with chain spec.
#[derive(Debug, Clone)]
struct TaikoEvmExecutor<EvmConfig> {
    /// The chainspec
    chain_spec: Arc<ChainSpec>,
    /// How to create an EVM.
    evm_config: EvmConfig,
}

impl<EvmConfig> TaikoEvmExecutor<EvmConfig>
where
    EvmConfig: ConfigureEvm,
{
    /// Executes the transactions in the block and returns the receipts of the transactions in the
    /// block, the total gas used and the list of EIP-7685 [requests](Request).
    ///
    /// This applies the pre-execution and post-execution changes that require an [EVM](Evm), and
    /// executes the transactions.
    ///
    /// # Note
    ///
    /// It does __not__ apply post-execution changes that do not require an [EVM](Evm), for that see
    /// [`EthBlockExecutor::post_execution`].
    fn execute_state_transitions<Ext, DB>(
        &self,
        block: &mut BlockWithSenders,
        mut evm: Evm<'_, Ext, &mut State<DB>>,
        enable_anchor: bool,
        enable_skip: bool,
    ) -> Result<TaikoExecuteOutput, BlockExecutionError>
    where
        DB: Database<Error = ProviderError>,
    {
        // apply pre execution changes
        apply_beacon_root_contract_call(
            &self.chain_spec,
            block.timestamp,
            block.number,
            block.parent_beacon_block_root,
            &mut evm,
        )?;
        apply_blockhashes_update(
            evm.db_mut(),
            &self.chain_spec,
            block.timestamp,
            block.number,
            block.parent_hash,
        )?;

        let treasury = self.chain_spec.treasury();

        // execute transactions
        let mut cumulative_gas_used = 0;
        let mut receipts: Vec<Receipt> = Vec::with_capacity(block.body.len());

        let delete_tx = |block: &mut BlockWithSenders, idx: usize| {
            block.body.remove(idx);
            block.senders.remove(idx);
        };
        if enable_anchor && block.body.is_empty() {
            return Err(ConsensusError::AnchorTxMissing.into());
        }
        let mut idx = 0;
        while idx < block.body.len() {
            let sender = block.senders[idx];
            let transaction = &block.body[idx];
            let is_anchor = idx == 0 && enable_anchor;

            // verify the anchor tx
            if is_anchor {
                check_anchor_tx(
                    transaction,
                    sender,
                    block.base_fee_per_gas.unwrap_or_default(),
                    treasury,
                )
                .map_err(|e| BlockExecutionError::CanonicalRevert { inner: e.to_string() })?;
            }

            // If the signature was not valid, the sender address will have been set to zero
            if sender == Address::ZERO {
                // Signature can be invalid if not taiko or not the anchor tx
                if !is_anchor && enable_skip {
                    // If the signature is not valid, skip the transaction
                    debug!(target: "taiko::executor", hash = ?transaction.hash(), "Invalid sender for tx");
                    delete_tx(block, idx);
                    continue;
                }
                // In all other cases, the tx needs to have a valid signature
                return Err(BlockExecutionError::CanonicalRevert {
                    inner: "invalid tx".to_string(),
                });
            }

            // The sum of the transaction's gas limit, Tg, and the gas utilized in this block prior,
            // must be no greater than the block's gasLimit.
            let block_available_gas = block.header.gas_limit - cumulative_gas_used;
            if transaction.gas_limit() > block_available_gas {
                if !is_anchor && enable_skip {
                    debug!(target: "taiko::executor", hash = ?transaction.hash(), want = ?transaction.gas_limit(), got = block_available_gas, "Invalid gas limit for tx");
                    delete_tx(block, idx);
                    continue;
                }
                return Err(BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                    transaction_gas_limit: transaction.gas_limit(),
                    block_available_gas,
                }
                .into());
            }

            EvmConfig::fill_tx_env(evm.tx_mut(), transaction, sender);

            // Set taiko specific data
            evm.tx_mut().taiko.is_anchor = is_anchor;
            // set the treasury address
            evm.tx_mut().taiko.treasury = treasury;
            if self.chain_spec.is_ontake_fork(block.number) {
                // set the basefee ratio
                evm.tx_mut().taiko.basefee_ratio = decode_ontake_extra_data(&block.extra_data);
            }

            // Execute transaction.
            let ResultAndState { result, state } = match evm.transact().map_err(move |err| {
                // Ensure hash is calculated for error log, if not already done
                BlockValidationError::EVM {
                    hash: transaction.recalculate_hash(),
                    error: err.into(),
                }
            }) {
                Ok(res) => res,
                Err(err) => {
                    if !is_anchor && enable_skip {
                        // Clear the state for the next tx
                        evm.context.evm.journaled_state = JournaledState::new(
                            evm.context.evm.journaled_state.spec,
                            HashSet::new(),
                        );
                        debug!(target: "taiko::executor", hash = ?transaction.hash(), error = ?err, "Invalid execute for tx");
                        delete_tx(block, idx);
                        continue;
                    }
                    return Err(err.into());
                }
            };

            evm.db_mut().commit(state);

            // append gas used
            cumulative_gas_used += result.gas_used();

            // Push transaction changeset and calculate header bloom filter for receipt.
            receipts.push(
                #[allow(clippy::needless_update)] // side-effect of optimism fields
                Receipt {
                    tx_type: transaction.tx_type(),
                    // Success flag was added in `EIP-658: Embedding transaction status code in
                    // receipts`.
                    success: result.is_success(),
                    cumulative_gas_used,
                    // convert to reth log
                    logs: result.into_logs(),
                    ..Default::default()
                },
            );

            idx += 1;
        }

        let requests = if self.chain_spec.is_prague_active_at_timestamp(block.timestamp) {
            // Collect all EIP-6110 deposits
            let deposit_requests =
                crate::eip6110::parse_deposits_from_receipts(&self.chain_spec, &receipts)?;

            // Collect all EIP-7685 requests
            let withdrawal_requests = apply_withdrawal_requests_contract_call(&mut evm)?;

            [deposit_requests, withdrawal_requests].concat()
        } else {
            vec![]
        };

        Ok(TaikoExecuteOutput { receipts, requests, gas_used: cumulative_gas_used })
    }
}

/// A basic Ethereum block executor.
///
/// Expected usage:
/// - Create a new instance of the executor.
/// - Execute the block.
#[derive(Debug)]
pub struct TaikoBlockExecutor<EvmConfig, DB> {
    /// Chain specific evm config that's used to execute a block.
    executor: TaikoEvmExecutor<EvmConfig>,
    /// The state to use for execution
    state: State<DB>,
}

impl<EvmConfig, DB> TaikoBlockExecutor<EvmConfig, DB> {
    /// Creates a new Ethereum block executor.
    pub const fn new(chain_spec: Arc<ChainSpec>, evm_config: EvmConfig, state: State<DB>) -> Self {
        Self { executor: TaikoEvmExecutor { chain_spec, evm_config }, state }
    }

    #[inline]
    fn chain_spec(&self) -> &ChainSpec {
        &self.executor.chain_spec
    }

    /// Returns mutable reference to the state that wraps the underlying database.
    #[allow(unused)]
    fn state_mut(&mut self) -> &mut State<DB> {
        &mut self.state
    }
}

fn encode_and_compress_tx_list(txs: &[TransactionSigned]) -> io::Result<Vec<u8>> {
    let encoded_buf = alloy_rlp::encode(TransactionSignedList(txs));
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&encoded_buf)?;
    encoder.finish()
}

impl<EvmConfig, DB> TaikoBlockExecutor<EvmConfig, DB>
where
    EvmConfig: ConfigureEvm,
    DB: Database<Error = ProviderError>,
{
    /// Configures a new evm configuration and block environment for the given block.
    ///
    /// # Caution
    ///
    /// This does not initialize the tx environment.
    fn evm_env_for_block(&self, header: &Header, total_difficulty: U256) -> EnvWithHandlerCfg {
        let mut cfg = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let mut block_env = BlockEnv::default();
        EvmConfig::fill_cfg_and_block_env(
            &mut cfg,
            &mut block_env,
            self.chain_spec(),
            header,
            total_difficulty,
        );

        EnvWithHandlerCfg::new_with_cfg_env(cfg, block_env, Default::default())
    }

    /// Execute a single block and apply the state changes to the internal state.
    ///
    /// Returns the receipts of the transactions in the block, the total gas used and the list of
    /// EIP-7685 [requests](Request).
    ///
    /// Returns an error if execution fails.
    fn execute_without_verification(
        &mut self,
        block: &mut BlockWithSenders,
        total_difficulty: U256,
        enable_anchor: bool,
        enable_skip: bool,
    ) -> Result<TaikoExecuteOutput, BlockExecutionError> {
        // 1. prepare state on new block
        self.on_new_block(&block.header);

        // 2. configure the evm and execute
        let env = self.evm_env_for_block(&block.header, total_difficulty);
        let output = {
            let evm = self.executor.evm_config.evm_with_env(&mut self.state, env);
            self.executor.execute_state_transitions(block, evm, enable_anchor, enable_skip)
        }?;

        // 3. apply post execution changes
        self.post_execution(block, total_difficulty)?;

        Ok(output)
    }

    fn build_transaction_list(
        &mut self,
        block: &BlockWithSenders,
        max_bytes_per_tx_list: u64,
        max_transactions_lists: u64,
    ) -> Result<Vec<TaskResult>, BlockExecutionError> {
        let env = self.evm_env_for_block(&block.header, U256::ZERO);
        let mut evm = self.executor.evm_config.evm_with_env(&mut self.state, env);
        // 2. configure the evm and execute
        // apply pre execution changes
        apply_beacon_root_contract_call(
            &self.executor.chain_spec,
            block.timestamp,
            block.number,
            block.parent_beacon_block_root,
            &mut evm,
        )?;

        apply_blockhashes_update(
            evm.db_mut(),
            &self.executor.chain_spec,
            block.timestamp,
            block.number,
            block.parent_hash,
        )?;

        let mut target_list: Vec<TaskResult> = vec![];
        // get previous env
        let previous_env = Box::new(evm.context.env().clone());

        for _ in 0..max_transactions_lists {
            // evm.context.evm.db.commit(state);
            // re-set the previous env
            evm.context.evm.env = previous_env.clone();

            let mut cumulative_gas_used = 0;
            let mut tx_list: Vec<TransactionSigned> = vec![];
            let mut buf_len: u64 = 0;

            for i in 0..block.body.len() {
                let transaction = block.body.get(i).unwrap();
                let sender = block.senders.get(i).unwrap();
                let block_available_gas = block.header.gas_limit - cumulative_gas_used;
                if transaction.gas_limit() > block_available_gas {
                    break;
                }

                EvmConfig::fill_tx_env(evm.tx_mut(), transaction, *sender);

                // Execute transaction.
                let ResultAndState { result, state } = match evm.transact() {
                    Ok(res) => res,
                    Err(_) => continue,
                };
                tx_list.push(transaction.clone());

                let compressed_buf =
                    encode_and_compress_tx_list(&tx_list).map_err(BlockExecutionError::other)?;
                if compressed_buf.len() > max_bytes_per_tx_list as usize {
                    tx_list.pop();
                    break;
                }

                buf_len = compressed_buf.len() as u64;
                // append gas used
                cumulative_gas_used += result.gas_used();

                // collect executed transaction state
                evm.db_mut().commit(state);
            }

            if tx_list.is_empty() {
                break;
            }
            target_list.push(TaskResult {
                txs: tx_list[..]
                    .iter()
                    .cloned()
                    .map(|tx| reth_rpc_types_compat::transaction::from_signed(tx).unwrap())
                    .collect(),
                estimated_gas_used: cumulative_gas_used,
                bytes_length: buf_len,
            });
        }

        Ok(target_list)
    }

    /// Apply settings before a new block is executed.
    pub(crate) fn on_new_block(&mut self, header: &Header) {
        // Set state clear flag if the block is after the Spurious Dragon hardfork.
        let state_clear_flag = self.chain_spec().is_spurious_dragon_active_at_block(header.number);
        self.state.set_state_clear_flag(state_clear_flag);
    }

    /// Apply post execution state changes that do not require an [EVM](Evm), such as: block
    /// rewards, withdrawals, and irregular DAO hardfork state change
    pub fn post_execution(
        &mut self,
        block: &BlockWithSenders,
        total_difficulty: U256,
    ) -> Result<(), BlockExecutionError> {
        let mut balance_increments = post_block_balance_increments(
            self.chain_spec(),
            block.number,
            block.difficulty,
            block.beneficiary,
            block.timestamp,
            total_difficulty,
            &block.ommers,
            block.withdrawals.as_ref().map(Withdrawals::as_ref),
        );

        // Irregular state change at Ethereum DAO hardfork
        if self.chain_spec().fork(Hardfork::Dao).transitions_at_block(block.number) {
            // drain balances from hardcoded addresses.
            let drained_balance: u128 = self
                .state
                .drain_balances(DAO_HARDKFORK_ACCOUNTS)
                .map_err(|_| BlockValidationError::IncrementBalanceFailed)?
                .into_iter()
                .sum();

            // return balance to DAO beneficiary.
            *balance_increments.entry(DAO_HARDFORK_BENEFICIARY).or_default() += drained_balance;
        }
        // increment balances
        self.state
            .increment_balances(balance_increments)
            .map_err(|_| BlockValidationError::IncrementBalanceFailed)?;

        Ok(())
    }
}

impl<EvmConfig, DB> Executor<DB> for TaikoBlockExecutor<EvmConfig, DB>
where
    EvmConfig: ConfigureEvm,
    DB: Database<Error = ProviderError>,
{
    type Input<'a> = BlockExecutionInput<'a, BlockWithSenders>;
    type Output = BlockExecutionOutput<Receipt>;
    type Error = BlockExecutionError;

    /// Executes the block and commits the state changes.
    ///
    /// Returns the receipts of the transactions in the block.
    ///
    /// Returns an error if the block could not be executed or failed verification.
    ///
    /// State changes are committed to the database.
    fn execute(mut self, input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        let BlockExecutionInput {
            block,
            total_difficulty,
            enable_anchor,
            enable_skip,
            enable_build,
            max_bytes_per_tx_list,
            max_transactions_lists,
        } = input;
        if enable_build {
            let target_list =
                self.build_transaction_list(block, max_bytes_per_tx_list, max_transactions_lists)?;
            Ok(BlockExecutionOutput {
                state: Default::default(),
                receipts: vec![],
                requests: vec![],
                gas_used: 0,
                target_list,
            })
        } else {
            let TaikoExecuteOutput { receipts, requests, gas_used } = self
                .execute_without_verification(
                    block,
                    total_difficulty,
                    enable_anchor,
                    enable_skip,
                )?;

            // NOTE: we need to merge keep the reverts for the bundle retention
            self.state.merge_transitions(BundleRetention::Reverts);
            Ok(BlockExecutionOutput {
                state: self.state.take_bundle(),
                receipts,
                requests,
                gas_used,
                target_list: vec![],
            })
        }
    }
}

/// An executor for a batch of blocks.
///
/// State changes are tracked until the executor is finalized.
#[derive(Debug)]
pub struct TaikoBatchExecutor<EvmConfig, DB> {
    /// The executor used to execute single blocks
    ///
    /// All state changes are committed to the [State].
    executor: TaikoBlockExecutor<EvmConfig, DB>,
    /// Keeps track of the batch and records receipts based on the configured prune mode
    batch_record: BlockBatchRecord,
    stats: BlockExecutorStats,
}

impl<EvmConfig, DB> TaikoBatchExecutor<EvmConfig, DB> {
    /// Returns mutable reference to the state that wraps the underlying database.
    #[allow(unused)]
    fn state_mut(&mut self) -> &mut State<DB> {
        self.executor.state_mut()
    }
}

impl<EvmConfig, DB> BatchExecutor<DB> for TaikoBatchExecutor<EvmConfig, DB>
where
    EvmConfig: ConfigureEvm,
    DB: Database<Error = ProviderError>,
{
    type Input<'a> = BlockExecutionInput<'a, BlockWithSenders>;
    type Output = ExecutionOutcome;
    type Error = BlockExecutionError;

    fn execute_and_verify_one(&mut self, input: Self::Input<'_>) -> Result<(), Self::Error> {
        let BlockExecutionInput { block, total_difficulty, enable_anchor, enable_skip, .. } = input;
        let TaikoExecuteOutput { receipts, requests, gas_used: _ } = self
            .executor
            .execute_without_verification(block, total_difficulty, enable_anchor, enable_skip)?;

        validate_block_post_execution(block, self.executor.chain_spec(), &receipts, &requests)?;

        // prepare the state according to the prune mode
        let retention = self.batch_record.bundle_retention(block.number);
        self.executor.state.merge_transitions(retention);

        // store receipts in the set
        self.batch_record.save_receipts(receipts)?;

        // store requests in the set
        self.batch_record.save_requests(requests);

        if self.batch_record.first_block().is_none() {
            self.batch_record.set_first_block(block.number);
        }

        Ok(())
    }

    fn finalize(mut self) -> Self::Output {
        self.stats.log_debug();

        ExecutionOutcome::new(
            self.executor.state.take_bundle(),
            self.batch_record.take_receipts(),
            self.batch_record.first_block().unwrap_or_default(),
            self.batch_record.take_requests(),
        )
    }

    fn set_tip(&mut self, tip: BlockNumber) {
        self.batch_record.set_tip(tip);
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.executor.state.bundle_state.size_hint())
    }
}
