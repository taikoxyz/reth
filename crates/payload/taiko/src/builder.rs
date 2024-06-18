use crate::consts::{anchor_selector, golden_touch, ANCHOR_GAS_LIMIT, TAIKO_L2_ADDRESS_SUFFIX};
use crate::error::TaikoPayloadBuilderError;
use reth_basic_payload_builder::*;
use reth_payload_builder::{
    error::PayloadBuilderError, TaikoBuiltPayload, TaikoPayloadBuilderAttributes,
};
use reth_primitives::L1Origin;
use reth_primitives::{
    constants::{BEACON_NONCE, EMPTY_RECEIPTS, EMPTY_TRANSACTIONS},
    eip4844::calculate_excess_blob_gas,
    hex::FromHex,
    proofs,
    revm::env::tx_env_with_recovered,
    Address, Block, Header, Receipt, Receipts, TransactionSigned, EMPTY_OMMER_ROOT_HASH, U256,
};
use reth_provider::{BundleStateWithReceipts, StateProviderFactory};
use reth_revm::database::StateProviderDatabase;
use reth_transaction_pool::{BestTransactionsAttributes, TransactionPool};
use revm::{
    db::states::bundle_state::BundleRetention,
    primitives::{EVMError, EnvWithHandlerCfg, ResultAndState},
    DatabaseCommit, StateBuilder,
};
use tracing::{debug, trace, warn};

/// Taiko's payload builder
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaikoPayloadBuilder;

/// Implementation of the [PayloadBuilder] trait for [TaikoPayloadBuilder].
impl<Pool, Client> PayloadBuilder<Pool, Client> for TaikoPayloadBuilder
where
    Client: StateProviderFactory,
    Pool: TransactionPool,
{
    type Attributes = TaikoPayloadBuilderAttributes;
    type BuiltPayload = TaikoBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, TaikoPayloadBuilderAttributes, TaikoBuiltPayload>,
    ) -> Result<BuildOutcome<TaikoBuiltPayload>, PayloadBuilderError> {
        taiko_payload_builder(args)
    }

    fn on_missing_payload(
        &self,
        _args: BuildArguments<Pool, Client, TaikoPayloadBuilderAttributes, TaikoBuiltPayload>,
    ) -> Option<TaikoBuiltPayload> {
        None
    }

    fn build_empty_payload(
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<TaikoBuiltPayload, PayloadBuilderError> {
        let extra_data = config.extra_data();
        let PayloadConfig {
            initialized_block_env,
            parent_block,
            attributes,
            chain_spec,
            initialized_cfg,
            ..
        } = config;

        debug!(target: "payload_builder", parent_hash = ?parent_block.hash(), parent_number = parent_block.number, "building empty payload");

        let state = client.state_by_block_hash(parent_block.hash()).map_err(|err| {
                warn!(target: "payload_builder", parent_hash=%parent_block.hash(), %err, "failed to get state for empty payload");
                err
            })?;
        let mut db = StateBuilder::new()
            .with_database_boxed(Box::new(StateProviderDatabase::new(&state)))
            .with_bundle_update()
            .build();

        let base_fee = initialized_block_env.basefee.to::<u64>();
        let block_number = initialized_block_env.number.to::<u64>();
        let block_gas_limit: u64 = initialized_block_env.gas_limit.try_into().unwrap_or(u64::MAX);

        // apply eip-4788 pre block contract call
        pre_block_beacon_root_contract_call(
                &mut db,
                &chain_spec,
                block_number,
                &initialized_cfg,
                &initialized_block_env,
                &attributes,
            ).map_err(|err| {
                warn!(target: "payload_builder", parent_hash=%parent_block.hash(), %err, "failed to apply beacon root contract call for empty payload");
                err
            })?;

        let WithdrawalsOutcome { withdrawals_root, withdrawals } =
                commit_withdrawals(&mut db, &chain_spec, attributes.payload_attributes.timestamp, attributes.payload_attributes.withdrawals.clone()).map_err(|err| {
                    warn!(target: "payload_builder", parent_hash=%parent_block.hash(), %err, "failed to commit withdrawals for empty payload");
                    err
                })?;

        // merge all transitions into bundle state, this would apply the withdrawal balance
        // changes and 4788 contract call
        db.merge_transitions(BundleRetention::PlainState);

        // calculate the state root
        let bundle_state =
            BundleStateWithReceipts::new(db.take_bundle(), Receipts::new(), block_number);
        let state_root = state.state_root(&bundle_state).map_err(|err| {
                warn!(target: "payload_builder", parent_hash=%parent_block.hash(), %err, "failed to calculate state root for empty payload");
                err
            })?;

        let mut excess_blob_gas = None;
        let mut blob_gas_used = None;

        if chain_spec.is_cancun_active_at_timestamp(attributes.payload_attributes.timestamp) {
            excess_blob_gas = if chain_spec.is_cancun_active_at_timestamp(parent_block.timestamp) {
                let parent_excess_blob_gas = parent_block.excess_blob_gas.unwrap_or_default();
                let parent_blob_gas_used = parent_block.blob_gas_used.unwrap_or_default();
                Some(calculate_excess_blob_gas(parent_excess_blob_gas, parent_blob_gas_used))
            } else {
                // for the first post-fork block, both parent.blob_gas_used and
                // parent.excess_blob_gas are evaluated as 0
                Some(calculate_excess_blob_gas(0, 0))
            };

            blob_gas_used = Some(0);
        }

        let header = Header {
            parent_hash: parent_block.hash(),
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: initialized_block_env.coinbase,
            state_root,
            transactions_root: EMPTY_TRANSACTIONS,
            withdrawals_root,
            receipts_root: EMPTY_RECEIPTS,
            logs_bloom: Default::default(),
            timestamp: attributes.payload_attributes.timestamp,
            mix_hash: attributes.payload_attributes.prev_randao,
            nonce: BEACON_NONCE,
            base_fee_per_gas: Some(base_fee),
            number: parent_block.number + 1,
            gas_limit: block_gas_limit,
            difficulty: U256::ZERO,
            gas_used: 0,
            extra_data,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root: attributes.payload_attributes.parent_beacon_block_root,
        };

        let block = Block { header, body: vec![], ommers: vec![], withdrawals };
        let sealed_block = block.seal_slow();

        Ok(TaikoBuiltPayload::new(
            attributes.payload_attributes.payload_id(),
            sealed_block,
            U256::ZERO,
            // chain_spec,
            // attributes,
        ))
    }
}

/// Constructs an Ethereum transaction payload using the best transactions from the pool.
///
/// Given build arguments including an Ethereum client, transaction pool,
/// and configuration, this function creates a transaction payload. Returns
/// a result indicating success with the payload or an error in case of failure.
#[inline]
pub(crate) fn taiko_payload_builder<Pool, Client>(
    args: BuildArguments<Pool, Client, TaikoPayloadBuilderAttributes, TaikoBuiltPayload>,
) -> Result<BuildOutcome<TaikoBuiltPayload>, PayloadBuilderError>
where
    Client: StateProviderFactory,
    Pool: TransactionPool,
{
    let BuildArguments { client, pool, mut cached_reads, config, cancel, best_payload } = args;

    let state_provider = client.state_by_block_hash(config.parent_block.hash())?;
    let state = StateProviderDatabase::new(&state_provider);
    let mut db = StateBuilder::new()
        .with_database_ref(cached_reads.as_db(&state))
        .with_bundle_update()
        .build();
    let extra_data = config.extra_data();
    let PayloadConfig {
        initialized_block_env,
        initialized_cfg,
        parent_block,
        attributes,
        chain_spec,
        ..
    } = config;

    debug!(target: "payload_builder", id=%attributes.payload_attributes.payload_id(), parent_hash = ?parent_block.hash(), parent_number = parent_block.number, "building new payload");
    let mut cumulative_gas_used = 0;
    let block_gas_limit: u64 = attributes
        .block_metadata
        .as_ref()
        .map(|block_metadata| block_metadata.gas_limit)
        .unwrap_or(u64::MAX);
    let base_fee = initialized_block_env.basefee.to::<u64>();

    let mut executed_txs = Vec::new();
    let best_txs = pool.best_transactions_with_attributes(BestTransactionsAttributes::new(
        base_fee,
        initialized_block_env.get_blob_gasprice().map(|gasprice| gasprice as u64),
    ));

    let total_fees = U256::ZERO;

    let block_number = initialized_block_env.number.to::<u64>();

    // apply eip-4788 pre block contract call
    pre_block_beacon_root_contract_call(
        &mut db,
        &chain_spec,
        block_number,
        &initialized_cfg,
        &initialized_block_env,
        &attributes,
    )?;

    let transactions = attributes
        .block_metadata
        .as_ref()
        .map(|bm| bm.tx_list.clone().unwrap_or_default())
        .unwrap_or(vec![]);

    let mut receipts = Vec::new();
    for (index, tx) in transactions.into_iter().enumerate() {
        // Check if the job was cancelled, if so we can exit early.
        if cancel.is_cancelled() {
            return Ok(BuildOutcome::Cancelled);
        }

        let mut tx = {
            let bytes = tx.to_vec();
            let mut bytes = bytes.as_slice();
            TransactionSigned::decode_enveloped(&mut bytes).map_err(|_| {
                PayloadBuilderError::Other(Box::new(TaikoPayloadBuilderError::FailedToDecodeTx))
            })?
        };

        if index == 0 {
            tx.transaction.mark_as_anchor().map_err(|_| {
                PayloadBuilderError::Other(Box::new(TaikoPayloadBuilderError::FailedToMarkAnchor))
            })?;
        }

        let tx = tx.try_into_ecrecovered().map_err(|_| {
            PayloadBuilderError::other(Box::new(
                TaikoPayloadBuilderError::TransactionEcRecoverFailed,
            ))
        })?;

        let mut evm = revm::Evm::builder()
            .with_db(&mut db)
            .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
                initialized_cfg.clone(),
                initialized_block_env.clone(),
                tx_env_with_recovered(&tx),
            ))
            .build();

        let ResultAndState { result, state } = match evm.transact() {
            Ok(res) => res,
            Err(err) => {
                match err {
                    EVMError::Transaction(err) => {
                        trace!(target: "payload_builder", %err, ?tx, "Error in sequencer transaction, skipping.");
                        continue;
                    }
                    err => {
                        // this is an error that we should treat as fatal for this attempt
                        return Err(PayloadBuilderError::EvmExecutionError(err));
                    }
                }
            }
        };

        // to release the db reference drop evm.
        drop(evm);
        // commit changes
        db.commit(state);

        let gas_used = result.gas_used();

        // add gas used by the transaction to cumulative gas used, before creating the receipt
        cumulative_gas_used += gas_used;

        // Push transaction changeset and calculate header bloom filter for receipt.
        receipts.push(Some(Receipt {
            tx_type: tx.tx_type(),
            success: result.is_success(),
            cumulative_gas_used,
            logs: result.into_logs().into_iter().map(Into::into).collect(),
        }));

        // append transaction to the list of executed transactions
        executed_txs.push(tx.into_signed());
    }

    // check if we have a better block
    if !is_better_payload(best_payload.as_ref(), total_fees) {
        // can skip building the block
        return Ok(BuildOutcome::Aborted { fees: total_fees, cached_reads });
    }

    let WithdrawalsOutcome { withdrawals_root, withdrawals } = commit_withdrawals(
        &mut db,
        &chain_spec,
        attributes.payload_attributes.timestamp,
        attributes.clone().payload_attributes.withdrawals,
    )?;

    // merge all transitions into bundle state, this would apply the withdrawal balance changes
    // and 4788 contract call
    db.merge_transitions(BundleRetention::PlainState);

    let bundle = BundleStateWithReceipts::new(
        db.take_bundle(),
        Receipts::from_vec(vec![receipts]),
        block_number,
    );
    let receipts_root = bundle.receipts_root_slow(block_number).expect("Number is in range");
    let logs_bloom = bundle.block_logs_bloom(block_number).expect("Number is in range");

    // calculate the state root
    let state_root = state_provider.state_root(&bundle)?;

    // create the block header
    let transactions_root = proofs::calculate_transaction_root(&executed_txs);

    // initialize empty blob sidecars. There are no blob transactions on L2.
    let blob_sidecars = Vec::new();
    let mut excess_blob_gas = None;
    let mut blob_gas_used = None;

    // only determine cancun fields when active
    if chain_spec.is_cancun_active_at_timestamp(attributes.payload_attributes.timestamp) {
        excess_blob_gas = if chain_spec.is_cancun_active_at_timestamp(parent_block.timestamp) {
            let parent_excess_blob_gas = parent_block.excess_blob_gas.unwrap_or_default();
            let parent_blob_gas_used = parent_block.blob_gas_used.unwrap_or_default();
            Some(calculate_excess_blob_gas(parent_excess_blob_gas, parent_blob_gas_used))
        } else {
            // for the first post-fork block, both parent.blob_gas_used and
            // parent.excess_blob_gas are evaluated as 0
            Some(calculate_excess_blob_gas(0, 0))
        };

        blob_gas_used = Some(0);
    }

    let header = Header {
        parent_hash: parent_block.hash(),
        ommers_hash: EMPTY_OMMER_ROOT_HASH,
        beneficiary: initialized_block_env.coinbase,
        state_root,
        transactions_root,
        receipts_root,
        withdrawals_root,
        logs_bloom,
        timestamp: attributes.payload_attributes.timestamp,
        mix_hash: attributes.payload_attributes.prev_randao,
        nonce: BEACON_NONCE,
        base_fee_per_gas: Some(base_fee),
        number: parent_block.number + 1,
        gas_limit: block_gas_limit,
        difficulty: U256::ZERO,
        gas_used: cumulative_gas_used,
        extra_data,
        parent_beacon_block_root: attributes.payload_attributes.parent_beacon_block_root,
        blob_gas_used,
        excess_blob_gas,
    };

    // Validate the anchor Tx
    if let Some(anchor) = executed_txs.first() {
        if !validate_anchor_tx(&anchor, &header) {
            return Err(PayloadBuilderError::Other(Box::new(
                TaikoPayloadBuilderError::InvalidAnchorTransaction,
            )));
        }
    }

    // seal the block
    let block = Block { header, body: executed_txs, ommers: vec![], withdrawals };

    let sealed_block = block.seal_slow();

    // L1Origin **MUST NOT** be nil, it's a required field in PayloadAttributesV1.
    let l1_origin = L1Origin {
        // Set the block hash before inserting the L1Origin into database.
        l2_block_hash: sealed_block.hash(),
        ..attributes.l1_origin.clone()
    };
    // Write L1Origin.
    client.insert_l1_origin(sealed_block.number, l1_origin)?;
    // Write the head L1Origin.
    client.insert_head_l1_origin(sealed_block.number)?;

    debug!(target: "payload_builder", ?sealed_block, "sealed built block");

    let mut payload =
        TaikoBuiltPayload::new(attributes.payload_attributes.id, sealed_block, total_fees);

    // extend the payload with the blob sidecars from the executed txs
    payload.extend_sidecars(blob_sidecars);

    Ok(BuildOutcome::Better { payload, cached_reads })
}

fn get_taiko_l2_address(chain_id: u64) -> Address {
    let prefix = chain_id.to_string();
    let zeros = "0".repeat(Address::len_bytes() * 2 - prefix.len() - TAIKO_L2_ADDRESS_SUFFIX.len());
    Address::from_hex(&format!("0x{prefix}{zeros}{TAIKO_L2_ADDRESS_SUFFIX}")).unwrap()
}

/// Checks if the given transaction is a valid TaikoL2.anchor transaction.
fn validate_anchor_tx(tx: &TransactionSigned, header: &Header) -> bool {
    if !tx.is_eip1559() {
        return false;
    }

    let Some(to) = tx.to() else {
        return false;
    };

    let Some(chain_id) = tx.chain_id() else {
        return false;
    };

    if to != get_taiko_l2_address(chain_id) {
        return false;
    }

    if !tx.input().starts_with(&anchor_selector().to_vec()) {
        return false;
    }

    if !tx.value().is_zero() {
        return false;
    }

    if tx.gas_limit() != ANCHOR_GAS_LIMIT {
        return false;
    }

    if let Some(base_fee_per_gas) = header.base_fee_per_gas {
        if tx.max_fee_per_gas() != base_fee_per_gas as u128 {
            return false;
        }
    }

    let Some(signer) = tx.recover_signer() else {
        return false;
    };

    signer == golden_touch()
}
