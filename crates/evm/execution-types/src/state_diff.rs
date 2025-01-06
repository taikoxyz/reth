use std::collections::HashMap;

use reth_primitives::{
    constants::{eip4844::MAX_DATA_GAS_PER_BLOCK, BEACON_NONCE}, eip4844::calculate_excess_blob_gas, proofs::{self, calculate_requests_root}, Block, BlockNumber, ChainId, EthereumHardforks, Header, Receipt, Receipts, Requests, StateDiff, StateDiffAccount, StateDiffStorageSlot, TransactionSigned, B256, EMPTY_OMMER_ROOT_HASH, U256
};
//use reth_provider::{StateProvider, StateProviderFactory};
//use reth_revm::database::{StateProviderDatabase, SyncStateProviderDatabase};
//use reth_transaction_pool::{BestTransactionsAttributes, TransactionPool};
use revm::{
    db::{states::bundle_state::BundleRetention, BundleAccount, BundleState, State},
    primitives::{AccountInfo, Bytecode, EVMError, EnvWithHandlerCfg, ResultAndState},
    DatabaseCommit, SyncDatabase,
};

use revm::primitives::ChainAddress;
use revm::db::AccountStatus;
use revm::db::states::StorageSlot;

use crate::{BlockExecutionOutput, ExecutionOutcome};


pub fn execution_outcome_to_state_diff(execution_outcome: &ExecutionOutcome, state_root: B256) -> StateDiff {
    assert_eq!(execution_outcome.receipts().len(), 1);
    let receipts = execution_outcome.receipts()[0].iter().map(|r| r.clone().unwrap()).collect();
    to_state_diff(&execution_outcome.bundle, &receipts, state_root)
}

pub fn block_execution_output_to_state_diff(block_execution_output: &BlockExecutionOutput<Receipt>, state_root: B256) -> StateDiff {
    to_state_diff(&block_execution_output.state, &block_execution_output.receipts, state_root)
}

pub fn to_state_diff(bundle_state: &BundleState, receipts: &Vec<Receipt>, state_root: B256) -> StateDiff {
    let mut state_diff = StateDiff {
        accounts: Vec::new(),
        receipts: receipts.clone(),
        gas_used: 0,
        state_root,
        transactions_root: B256::ZERO,
    };
    for (address, bundle_account) in bundle_state.state.iter() {
        let storage = bundle_account.storage.iter().map(|(&key, value)| StateDiffStorageSlot {
            key, value: value.present_value
        }).collect();

        let account_info = bundle_account.account_info().unwrap_or_default();
        let state_diff_account = StateDiffAccount {
            address: address.1,
            storage,
            balance: account_info.balance,
            nonce: account_info.nonce,
            code_hash: account_info.code_hash,
            code: account_info.code.unwrap_or_default().bytes(),
        };
        state_diff.accounts.push(state_diff_account);
    }
    state_diff
}

pub fn state_diff_to_block_execution_output(chain_id: u64, state_diff: &StateDiff) -> BlockExecutionOutput<Receipt> {
    let mut block_execution_output = BlockExecutionOutput::<Receipt> {
        state: BundleState::default(),
        receipts: state_diff.receipts.clone(),
        requests: Vec::new(),
        gas_used: 0,
    };
    for account in state_diff.accounts.iter() {

        let mut new_account = BundleAccount {
            info: Some(AccountInfo {
                balance: account.balance,
                nonce: account.nonce,
                code_hash: account.code_hash,
                code: Some(Bytecode::new_raw_checked(account.code.clone()).unwrap()),
            }),
            original_info: None,
            storage: HashMap::new(),
            status: AccountStatus::Changed,
        };
        for storage_slot in account.storage.iter() {
            new_account.storage.insert(storage_slot.key, StorageSlot {
                previous_or_original_value: storage_slot.value,
                present_value: storage_slot.value,
            });
        }

        block_execution_output.state.state.insert(ChainAddress(chain_id, account.address), new_account);
    }
    block_execution_output
}
