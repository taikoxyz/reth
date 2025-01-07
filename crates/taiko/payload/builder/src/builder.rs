//! Taiko's payload builder module.

use std::sync::Arc;

use crate::error::TaikoPayloadBuilderError;
use alloy_consensus::EMPTY_OMMER_ROOT_HASH;
use alloy_eips::merge::BEACON_NONCE;
use alloy_primitives::{Bloom, U256};
use reth_basic_payload_builder::*;
use reth_chain_state::ExecutedBlock;
use reth_errors::{BlockExecutionError, BlockValidationError};
use reth_evm::{
    execute::{BasicBlockExecutorProvider, BlockExecutionOutput, BlockExecutorProvider, Executor},
    ConfigureEvm, NextBlockEnvAttributes,
};
use reth_payload_builder::EthBuiltPayload;
use reth_payload_primitives::{PayloadBuilderAttributes, PayloadBuilderError};
use reth_primitives::{
    proofs, Block, BlockBody, BlockExt, EthereumHardforks, Header, TransactionSigned,
};
use reth_provider::{
    BlockExecutionInput, ChainSpecProvider, ExecutionOutcome, L1OriginWriter, StateProviderFactory,
};
use reth_revm::{
    database::StateProviderDatabase,
    primitives::{BlockEnv, CfgEnvWithHandlerCfg},
};
use reth_taiko_chainspec::TaikoChainSpec;
use reth_taiko_engine_primitives::TaikoPayloadBuilderAttributes;
use reth_taiko_evm::{TaikoEvmConfig, TaikoExecutionStrategyFactory, TaikoExecutorProvider};
use reth_taiko_primitives::L1Origin;
use reth_transaction_pool::{noop::NoopTransactionPool, PoolTransaction, TransactionPool};
use revm_primitives::calc_excess_blob_gas;
use tracing::{debug, warn};

/// Taiko's payload builder
#[derive(Debug, Clone)]
pub struct TaikoPayloadBuilder<EvmConfig = TaikoEvmConfig> {
    evm_config: EvmConfig,
    /// The type responsible for creating the evm.
    block_executor: BasicBlockExecutorProvider<TaikoExecutionStrategyFactory<EvmConfig>>,
}

impl TaikoPayloadBuilder {
    /// `TaikoPayloadBuilder` constructor.
    pub fn new(evm_config: TaikoEvmConfig, chain_spec: Arc<TaikoChainSpec>) -> Self {
        let block_executor = TaikoExecutorProvider::taiko(chain_spec);
        Self { block_executor, evm_config }
    }
}

impl<EvmConfig> TaikoPayloadBuilder<EvmConfig>
where
    EvmConfig: ConfigureEvm<Header = Header>,
{
    /// Returns the configured [`CfgEnvWithHandlerCfg`] and [`BlockEnv`] for the targeted payload
    /// (that has the `parent` as its parent).
    fn cfg_and_block_env(
        &self,
        config: &PayloadConfig<TaikoPayloadBuilderAttributes>,
        parent: &Header,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), EvmConfig::Error> {
        let next_attributes = NextBlockEnvAttributes {
            timestamp: config.attributes.timestamp(),
            suggested_fee_recipient: config.attributes.suggested_fee_recipient(),
            prev_randao: config.attributes.prev_randao(),
        };
        let (cfg_with_handler_cfg, mut block_env) =
            self.evm_config.next_cfg_and_block_env(parent, next_attributes)?;
        block_env.basefee = config.attributes.base_fee_per_gas;
        block_env.gas_limit = U256::from(config.attributes.block_metadata.gas_limit);
        block_env.prevrandao = Some(config.attributes.block_metadata.mix_hash);
        block_env.coinbase = config.attributes.block_metadata.beneficiary;

        Ok((cfg_with_handler_cfg, block_env))
    }
}

/// Implementation of the [`PayloadBuilder`] trait for [`TaikoPayloadBuilder`].
impl<Pool, Client, EvmConfig> PayloadBuilder<Pool, Client> for TaikoPayloadBuilder<EvmConfig>
where
    EvmConfig: ConfigureEvm<Header = Header, Transaction = TransactionSigned>,
    Client: StateProviderFactory + ChainSpecProvider<ChainSpec = TaikoChainSpec> + L1OriginWriter,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>,
{
    type Attributes = TaikoPayloadBuilderAttributes;
    type BuiltPayload = EthBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, TaikoPayloadBuilderAttributes, EthBuiltPayload>,
    ) -> Result<BuildOutcome<EthBuiltPayload>, PayloadBuilderError> {
        let BuildArguments { cached_reads, .. } = args;
        Ok(BuildOutcome::Aborted { fees: U256::ZERO, cached_reads })
    }

    fn build_empty_payload(
        &self,
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<EthBuiltPayload, PayloadBuilderError> {
        let args = BuildArguments::new(
            client,
            // we use defaults here because for the empty payload we don't need to execute anything
            NoopTransactionPool::default(),
            Default::default(),
            config,
            Default::default(),
            None,
        );
        let (_cfg_env, block_env) = self
            .cfg_and_block_env(&args.config, &args.config.parent_header)
            .map_err(PayloadBuilderError::other)?;

        taiko_payload_builder(&self.block_executor, args, block_env)
    }
}

/// Constructs an Ethereum transaction payload using the best transactions from the pool.
///
/// Given build arguments including an Ethereum client, transaction pool,
/// and configuration, this function creates a transaction payload. Returns
/// a result indicating success with the payload or an error in case of failure.
#[inline]
fn taiko_payload_builder<EvmConfig, Pool, Client>(
    executor: &BasicBlockExecutorProvider<TaikoExecutionStrategyFactory<EvmConfig>>,
    args: BuildArguments<Pool, Client, TaikoPayloadBuilderAttributes, EthBuiltPayload>,
    initialized_block_env: BlockEnv,
) -> Result<EthBuiltPayload, PayloadBuilderError>
where
    EvmConfig: ConfigureEvm<Header = Header, Transaction = TransactionSigned>,
    Client: StateProviderFactory + ChainSpecProvider<ChainSpec = TaikoChainSpec> + L1OriginWriter,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>,
{
    let BuildArguments { client, pool: _, mut cached_reads, config, cancel: _, best_payload: _ } =
        args;
    let chain_spec = client.chain_spec();
    let state_provider = client.state_by_block_hash(config.parent_header.hash())?;
    let state = StateProviderDatabase::new(state_provider);
    let mut db = cached_reads.as_db_mut(state);
    let PayloadConfig { parent_header, attributes, extra_data: _ } = config;

    debug!(target: "taiko_payload_builder", id=%attributes.payload_attributes.payload_id(), parent_hash = ?parent_header.hash(), parent_number = parent_header.number, "building new payload");
    let block_gas_limit: u64 = initialized_block_env.gas_limit.try_into().unwrap_or(u64::MAX);
    let base_fee = initialized_block_env.basefee.to::<u64>();

    let block_number = initialized_block_env.number.to::<u64>();

    let transactions: Vec<TransactionSigned> =
        alloy_rlp::Decodable::decode(&mut attributes.block_metadata.tx_list.as_ref())
            .map_err(|_| PayloadBuilderError::other(TaikoPayloadBuilderError::FailedToDecodeTx))?;

    let blob_gas_used =
        chain_spec.is_cancun_active_at_timestamp(attributes.timestamp()).then(|| {
            let mut sum_blob_gas_used = 0;
            for tx in &transactions {
                if let Some(blob_tx) = tx.transaction.as_eip4844() {
                    sum_blob_gas_used += blob_tx.blob_gas();
                }
            }
            sum_blob_gas_used
        });

    // if shanghai is active, include empty withdrawals
    let withdrawals = chain_spec
        .is_shanghai_active_at_timestamp(attributes.payload_attributes.timestamp)
        .then_some(attributes.payload_attributes.withdrawals);

    let mut header = Header {
        parent_hash: parent_header.hash(),
        ommers_hash: EMPTY_OMMER_ROOT_HASH,
        beneficiary: initialized_block_env.coinbase,
        state_root: Default::default(),
        transactions_root: Default::default(),
        receipts_root: Default::default(),
        withdrawals_root: withdrawals.as_ref().map(|w| proofs::calculate_withdrawals_root(w)),
        logs_bloom: Default::default(),
        timestamp: attributes.payload_attributes.timestamp,
        mix_hash: initialized_block_env.prevrandao.unwrap(),
        nonce: BEACON_NONCE.into(),
        base_fee_per_gas: Some(base_fee),
        number: block_number,
        gas_limit: block_gas_limit,
        difficulty: initialized_block_env.difficulty,
        gas_used: 0,
        extra_data: attributes.block_metadata.extra_data.clone().into(),
        parent_beacon_block_root: attributes.payload_attributes.parent_beacon_block_root,
        blob_gas_used,
        excess_blob_gas: None,
        requests_hash: None,
        target_blobs_per_block: None,
    };

    if chain_spec.is_cancun_active_at_timestamp(attributes.payload_attributes.timestamp) {
        header.parent_beacon_block_root = parent_header.parent_beacon_block_root;
        header.blob_gas_used = Some(0);

        let (parent_excess_blob_gas, parent_blob_gas_used) =
            if chain_spec.is_cancun_active_at_timestamp(parent_header.timestamp) {
                (
                    parent_header.excess_blob_gas.unwrap_or_default(),
                    parent_header.blob_gas_used.unwrap_or_default(),
                )
            } else {
                (0, 0)
            };

        header.excess_blob_gas =
            Some(calc_excess_blob_gas(parent_excess_blob_gas, parent_blob_gas_used))
    }

    // seal the block
    let mut block = Block { header, body: BlockBody { transactions, ommers: vec![], withdrawals } }
        .with_recovered_senders()
        .ok_or(BlockExecutionError::Validation(BlockValidationError::SenderRecoveryError))?;

    // execute the block
    let block_input = BlockExecutionInput {
        block: &block,
        total_difficulty: U256::ZERO,
        enable_anchor: true,
        enable_skip: true,
    };
    // execute the block
    let output = executor.executor(&mut db).execute(block_input)?;
    output.apply_skip(&mut block);
    let BlockExecutionOutput { state, receipts, requests, gas_used, .. } = output;
    let execution_outcome =
        ExecutionOutcome::new(state, receipts.into(), block.number, vec![requests.clone()]);

    // if prague is active, include empty requests
    let requests = chain_spec
        .is_prague_active_at_timestamp(attributes.payload_attributes.timestamp)
        .then_some(requests);
    block.header.requests_hash = requests.as_ref().map(|r| r.requests_hash());
    // now we need to update certain header fields with the results of the execution
    block.header.transactions_root = proofs::calculate_transaction_root(&block.body.transactions);
    let hashed_state = db.inner().hashed_post_state(execution_outcome.state());
    let (state_root, trie_output) = {
        db.inner().state_root_with_updates(hashed_state.clone()).inspect_err(|err| {
            warn!(target: "payload_builder",
                parent_hash=%parent_header.hash(),
                %err,
                "failed to calculate state root for payload"
            );
        })?
    };
    block.header.state_root = state_root;
    block.header.gas_used = gas_used;

    let receipts = execution_outcome.receipts_by_block(block.header.number);

    // update logs bloom
    let receipts_with_bloom =
        receipts.iter().map(|r| r.as_ref().unwrap().bloom_slow()).collect::<Vec<Bloom>>();
    block.header.logs_bloom = receipts_with_bloom.iter().fold(Bloom::ZERO, |bloom, r| bloom | *r);

    // update receipts root
    block.header.receipts_root =
        execution_outcome.receipts_root_slow(block.header.number).expect("Receipts is present");

    let sealed_block = Arc::new(block.block.seal_slow());

    if attributes.l1_origin.batch_id.is_none() &&
        (attributes.l1_origin.l1_block_hash.is_none() ||
            attributes.l1_origin.l1_block_height.is_none())
    {
        return Err(PayloadBuilderError::other(TaikoPayloadBuilderError::MissingL1Origin));
    }

    // L1Origin **MUST NOT** be nil, it's a required field in PayloadAttributesV1.
    let l1_origin = L1Origin {
        // Set the block hash before inserting the L1Origin into database.
        l2_block_hash: sealed_block.hash(),
        ..attributes.l1_origin
    };
    debug!(target: "taiko_payload_builder", ?l1_origin, "save l1 origin");
    let block_id = l1_origin.block_id.try_into().unwrap();
    // Write L1Origin and head L1Origin.
    client.save_l1_origin(block_id, l1_origin)?;

    debug!(target: "taiko_payload_builder", ?sealed_block, "sealed built block");

    // create the executed block data
    let executed = ExecutedBlock {
        block: sealed_block.clone(),
        senders: Arc::new(block.senders),
        execution_output: Arc::new(execution_outcome),
        hashed_state: Arc::new(hashed_state),
        trie: Arc::new(trie_output),
    };
    let payload = EthBuiltPayload::new(
        attributes.payload_attributes.id,
        sealed_block,
        U256::ZERO,
        Some(executed),
        requests,
    );
    Ok(payload)
}
