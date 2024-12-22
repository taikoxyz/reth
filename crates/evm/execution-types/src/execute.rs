use alloy_primitives::U256;
use alloy_rpc_types_eth::transaction::Transaction;
use reth_primitives::Request;
use revm::db::BundleState;

/// A helper type for ethereum block inputs that consists of a block and the total difficulty.
#[derive(Debug)]
pub struct BlockExecutionInput<'a, Block> {
    /// The block to execute.
    pub block: &'a mut Block,
    /// The total difficulty of the block.
    pub total_difficulty: U256,
    /// Enable anchor transaction.
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
    pub fn new(block: &'a mut Block, total_difficulty: U256) -> Self {
        Self {
            block,
            total_difficulty,
            enable_anchor: false,
            enable_skip: true,
            enable_build: false,
            max_bytes_per_tx_list: 0,
            max_transactions_lists: 0,
        }
    }
}

impl<'a, Block> From<(&'a mut Block, U256)> for BlockExecutionInput<'a, Block> {
    fn from((block, total_difficulty): (&'a mut Block, U256)) -> Self {
        Self::new(block, total_difficulty)
    }
}

/// Result of the trigger
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// Transactions
    pub txs: Vec<Transaction>,
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
    /// All the EIP-7685 requests of the transactions in the block.
    pub requests: Vec<Request>,
    /// The total gas used by the block.
    pub gas_used: u64,
    /// The target list.
    pub target_list: Vec<TaskResult>,
}
