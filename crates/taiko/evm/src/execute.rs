//! Optimism block execution strategy.

use crate::TaikoEvmConfig;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Transaction as _};
use alloy_eips::eip7685::Requests;
use alloy_rlp::Encodable;
use core::fmt::Display;
use flate2::{write::ZlibEncoder, Compression};
use reth_chainspec::{EthereumHardfork, EthereumHardforks};
use reth_consensus::ConsensusError;
use reth_ethereum_consensus::validate_block_post_execution;
use reth_evm::{
    execute::{
        balance_increment_state, BasicBlockExecutorProvider, BlockExecutionError,
        BlockExecutionStrategy, BlockExecutionStrategyFactory, BlockValidationError, ExecuteOutput,
        ProviderError,
    },
    state_change::post_block_balance_increments,
    system_calls::{OnStateHook, SystemCaller},
    ConfigureEvm, EnvExt, TxEnvOverrides,
};
use reth_evm_ethereum::dao_fork::{DAO_HARDFORK_BENEFICIARY, DAO_HARDKFORK_ACCOUNTS};
use reth_execution_types::BlockExecutionInput;
use reth_primitives::{BlockWithSenders, EthPrimitives, Receipt, TransactionSigned};
use reth_revm::{Database, State};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_consensus::check_anchor_tx;
use revm::{interpreter::Host, JournaledState};
use revm_primitives::{db::DatabaseCommit, EnvWithHandlerCfg, HashSet, ResultAndState, U256};
use std::io::{self, Write};
use tracing::debug;

/// Factory for [`OpExecutionStrategy`].
#[derive(Debug, Clone)]
pub struct TaikoExecutionStrategyFactory<EvmConfig = TaikoEvmConfig> {
    /// The chainspec
    chain_spec: Arc<TaikoChainSpec>,
    /// How to create an EVM.
    evm_config: EvmConfig,
}

impl TaikoExecutionStrategyFactory {
    /// Creates a new default taiko executor strategy factory.
    pub fn taiko(chain_spec: Arc<TaikoChainSpec>) -> Self {
        Self::new(chain_spec.clone(), TaikoEvmConfig::new(chain_spec))
    }
}

impl<EvmConfig> TaikoExecutionStrategyFactory<EvmConfig> {
    /// Creates a new executor strategy factory.
    pub const fn new(chain_spec: Arc<TaikoChainSpec>, evm_config: EvmConfig) -> Self {
        Self { chain_spec, evm_config }
    }
}

impl<EvmConfig> BlockExecutionStrategyFactory for TaikoExecutionStrategyFactory<EvmConfig>
where
    EvmConfig: Clone
        + Unpin
        + Sync
        + Send
        + 'static
        + ConfigureEvm<Header = alloy_consensus::Header, Transaction = TransactionSigned>,
{
    type Primitives = EthPrimitives;
    type Strategy<DB: Database<Error: Into<ProviderError> + Display>> =
        TaikoExecutionStrategy<DB, EvmConfig>;

    fn create_strategy<DB>(&self, db: DB) -> Self::Strategy<DB>
    where
        DB: Database<Error: Into<ProviderError> + Display>,
    {
        let state =
            State::builder().with_database(db).with_bundle_update().without_state_clear().build();
        TaikoExecutionStrategy::new(state, self.chain_spec.clone(), self.evm_config.clone())
    }
}

/// Block execution strategy for Optimism.
#[allow(missing_debug_implementations)]
pub struct TaikoExecutionStrategy<DB, EvmConfig>
where
    EvmConfig: Clone,
{
    /// The chainspec
    chain_spec: Arc<TaikoChainSpec>,
    /// How to create an EVM.
    evm_config: EvmConfig,
    /// Optional overrides for the transactions environment.
    tx_env_overrides: Option<Box<dyn TxEnvOverrides>>,
    /// Current state for block execution.
    state: State<DB>,
    /// Utility to call system smart contracts.
    system_caller: SystemCaller<EvmConfig, TaikoChainSpec>,
}

impl<DB, EvmConfig> TaikoExecutionStrategy<DB, EvmConfig>
where
    EvmConfig: Clone,
{
    /// Creates a new [`TaikoExecutionStrategy`]
    pub fn new(state: State<DB>, chain_spec: Arc<TaikoChainSpec>, evm_config: EvmConfig) -> Self {
        let system_caller = SystemCaller::new(evm_config.clone(), chain_spec.clone());
        Self { state, chain_spec, evm_config, system_caller, tx_env_overrides: None }
    }
}

impl<DB, EvmConfig> TaikoExecutionStrategy<DB, EvmConfig>
where
    DB: Database<Error: Into<ProviderError> + Display>,
    EvmConfig: ConfigureEvm<Header = alloy_consensus::Header>,
{
    /// Configures a new evm configuration and block environment for the given block.
    ///
    /// Caution: this does not initialize the tx environment.
    fn evm_env_for_block(&self, header: &Header, total_difficulty: U256) -> EnvWithHandlerCfg {
        let (cfg, block_env) = self.evm_config.cfg_and_block_env(header, total_difficulty);
        EnvWithHandlerCfg::new_with_cfg_env(cfg, block_env, Default::default())
    }
}

impl<DB, EvmConfig> BlockExecutionStrategy for TaikoExecutionStrategy<DB, EvmConfig>
where
    DB: Database<Error: Into<ProviderError> + Display>,
    EvmConfig: ConfigureEvm<Header = alloy_consensus::Header, Transaction = TransactionSigned>,
{
    type DB = DB;
    type Error = BlockExecutionError;

    type Primitives = EthPrimitives;

    fn init(&mut self, tx_env_overrides: Box<dyn TxEnvOverrides>) {
        self.tx_env_overrides = Some(tx_env_overrides);
    }

    fn apply_pre_execution_changes(
        &mut self,
        block: &BlockWithSenders,
        total_difficulty: U256,
    ) -> Result<(), Self::Error> {
        // Set state clear flag if the block is after the Spurious Dragon hardfork.
        let state_clear_flag =
            (*self.chain_spec).is_spurious_dragon_active_at_block(block.header.number);
        self.state.set_state_clear_flag(state_clear_flag);

        let env = self.evm_env_for_block(&block.header, total_difficulty);
        let mut evm = self.evm_config.evm_with_env(&mut self.state, env);

        self.system_caller.apply_pre_execution_changes(&block.block, &mut evm)?;

        Ok(())
    }

    fn execute_transactions(
        &mut self,
        input: BlockExecutionInput<'_, BlockWithSenders>,
    ) -> Result<ExecuteOutput<Receipt>, Self::Error> {
        let BlockExecutionInput { block, total_difficulty, enable_anchor, enable_skip } = input;

        let env = self.evm_env_for_block(&block.header, total_difficulty);
        let mut evm = self.evm_config.evm_with_env(&mut self.state, env);
        let mut cumulative_gas_used = 0;
        let mut receipts = Vec::with_capacity(block.body.transactions.len());
        let mut skipped_list = Vec::with_capacity(block.body.transactions.len());
        let treasury = self.chain_spec.treasury();

        for (idx, (sender, transaction)) in block.transactions_with_sender().enumerate() {
            let is_anchor = idx == 0 && enable_anchor;

            // verify the anchor tx
            if is_anchor {
                check_anchor_tx(
                    transaction,
                    *sender,
                    block.base_fee_per_gas.unwrap_or_default(),
                    treasury,
                )
                .map_err(|e| BlockValidationError::AnchorValidation { message: e.to_string() })?;
            }

            // The sum of the transaction’s gas limit, Tg, and the gas utilized in this block prior,
            // must be no greater than the block’s gasLimit.
            let block_available_gas = block.header.gas_limit - cumulative_gas_used;
            if transaction.gas_limit() > block_available_gas {
                if !is_anchor && enable_skip {
                    debug!(target: "taiko::executor", hash = ?transaction.hash(), want = ?transaction.gas_limit(), got = block_available_gas, "Invalid gas limit for tx");
                    skipped_list.push(idx);
                    continue;
                }
                return Err(BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                    transaction_gas_limit: transaction.gas_limit(),
                    block_available_gas,
                }
                .into());
            }

            self.evm_config.fill_tx_env(
                evm.tx_mut(),
                transaction,
                *sender,
                Some(EnvExt {
                    is_anchor,
                    block_number: block.number,
                    extra_data: &block.extra_data,
                }),
            );

            if let Some(tx_env_overrides) = &mut self.tx_env_overrides {
                tx_env_overrides.apply(evm.tx_mut());
            }

            // Execute transaction.
            let result_and_state = match evm.transact().map_err(move |err| {
                let new_err = err.map_db_err(|e| e.into());
                // Ensure hash is calculated for error log, if not already done
                BlockValidationError::EVM {
                    hash: transaction.recalculate_hash(),
                    error: Box::new(new_err),
                }
            }) {
                Ok(res) => res,
                Err(err) => {
                    if !is_anchor && enable_skip {
                        // Clear the state for the next tx
                        evm.context.evm.journaled_state = JournaledState::new(
                            evm.context.evm.journaled_state.spec,
                            HashSet::default(),
                        );
                        debug!(target: "taiko::executor", hash = ?transaction.hash(), error = ?err, "Invalid execute for tx");
                        skipped_list.push(idx);
                        continue;
                    }
                    return Err(err.into());
                }
            };

            self.system_caller.on_state(&result_and_state.state);
            let ResultAndState { result, state } = result_and_state;
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
        }
        Ok(ExecuteOutput { receipts, gas_used: cumulative_gas_used, skipped_list })
    }

    fn apply_post_execution_changes(
        &mut self,
        block: &BlockWithSenders,
        total_difficulty: U256,
        receipts: &[Receipt],
    ) -> Result<Requests, Self::Error> {
        let env = self.evm_env_for_block(&block.header, total_difficulty);
        let mut evm = self.evm_config.evm_with_env(&mut self.state, env);

        let requests = if self.chain_spec.is_prague_active_at_timestamp(block.timestamp) {
            // Collect all EIP-6110 deposits
            let deposit_requests = reth_evm_ethereum::eip6110::parse_deposits_from_receipts(
                &self.chain_spec,
                receipts,
            )?;

            let mut requests = Requests::default();

            if !deposit_requests.is_empty() {
                requests.push_request(core::iter::once(0).chain(deposit_requests).collect());
            }

            requests.extend(self.system_caller.apply_post_execution_changes(&mut evm)?);
            requests
        } else {
            Requests::default()
        };
        drop(evm);

        let mut balance_increments =
            post_block_balance_increments(&self.chain_spec, &block.block, total_difficulty);

        // Irregular state change at Ethereum DAO hardfork
        if self.chain_spec.fork(EthereumHardfork::Dao).transitions_at_block(block.number) {
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
            .increment_balances(balance_increments.clone())
            .map_err(|_| BlockValidationError::IncrementBalanceFailed)?;
        // call state hook with changes due to balance increments.
        let balance_state = balance_increment_state(&balance_increments, &mut self.state)?;
        self.system_caller.on_state(&balance_state);

        Ok(requests)
    }

    fn state_ref(&self) -> &State<DB> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<DB> {
        &mut self.state
    }

    fn with_state_hook(&mut self, hook: Option<Box<dyn OnStateHook>>) {
        self.system_caller.with_state_hook(hook);
    }

    fn validate_block_post_execution(
        &self,
        block: &BlockWithSenders,
        receipts: &[Receipt],
        requests: &Requests,
    ) -> Result<(), ConsensusError> {
        validate_block_post_execution(block, &self.chain_spec.clone(), receipts, requests)
    }
}

/// Helper type with backwards compatible methods to obtain executor providers.
#[derive(Debug)]
pub struct TaikoExecutorProvider;

impl TaikoExecutorProvider {
    /// Creates a new default optimism executor strategy factory.
    pub fn taiko(
        chain_spec: Arc<TaikoChainSpec>,
    ) -> BasicBlockExecutorProvider<TaikoExecutionStrategyFactory> {
        BasicBlockExecutorProvider::new(TaikoExecutionStrategyFactory::taiko(chain_spec))
    }
}

/// Encode and compress a list of transactions.
pub fn encode_and_compress_tx_list<T: Encodable>(txs: &Vec<T>) -> io::Result<Vec<u8>> {
    let encoded_buf = alloy_rlp::encode(txs);
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&encoded_buf)?;
    encoder.finish()
}
