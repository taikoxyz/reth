use alloy_eips::eip7685::Requests;
use alloy_primitives::U256;
use reth_primitives::{BlockWithSenders, TransactionSigned};
use revm::db::BundleState;

/// A helper type for ethereum block inputs that consists of a block and the total difficulty.
#[derive(Debug, Clone, Copy)]
pub struct BlockExecutionInput<'a, Block> {
    /// The block to execute.
    pub block: &'a Block,
    /// The total difficulty of the block.
    pub total_difficulty: U256,
    /// Enable anchor transaction. Default is true.
    pub enable_anchor: bool,
    /// Enable skip invalid transaction.
    pub enable_skip: bool,
    /// Enable build transaction lists.
    pub enable_build: bool,
    /// Max compressed bytes.
    pub max_bytes_per_tx_list: u64,
    /// Max length of transactions list.
    pub max_transactions_lists: u64,
}

impl<'a, Block> BlockExecutionInput<'a, Block> {
    /// Creates a new input.
    pub const fn new(block: &'a Block, total_difficulty: U256) -> Self {
        Self {
            block,
            total_difficulty,
            enable_anchor: true,
            enable_skip: false,
            enable_build: false,
            max_bytes_per_tx_list: 0,
            max_transactions_lists: 0,
        }
    }
}

impl<'a, Block> From<(&'a Block, U256)> for BlockExecutionInput<'a, Block> {
    fn from((block, total_difficulty): (&'a Block, U256)) -> Self {
        Self::new(block, total_difficulty)
    }
}

/// Result of the trigger
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// Transactions
    pub txs: Vec<TransactionSigned>,
    /// Estimated gas used
    pub estimated_gas_used: u64,
    /// Bytes length
    pub bytes_length: u64,
}

/// The output of an ethereum block.
///
/// Contains the state changes, transaction receipts, and total gas used in the block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExecutionOutput<T> {
    /// The changed state of the block after execution.
    pub state: BundleState,
    /// All the receipts of the transactions in the block.
    pub receipts: Vec<T>,
    /// All the EIP-7685 requests in the block.
    pub requests: Requests,
    /// The total gas used by the block.
    pub gas_used: u64,
    /// The target list.
    pub target_list: Vec<TaskResult>,
    /// The skipped transactions when `BlockExecutionInput::enable_skip`.
    pub skipped_list: Vec<usize>,
}

impl<T> BlockExecutionOutput<T> {
    /// Remote the skipped transactions from the block.
    pub fn apply_skip(&self, block: &mut BlockWithSenders) {
        for i in &self.skipped_list {
            block.senders.remove(*i);
            block.body.transactions.remove(*i);
        }
    }
}
