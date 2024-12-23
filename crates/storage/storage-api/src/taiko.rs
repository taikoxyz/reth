use alloy_primitives::BlockNumber;
use reth_storage_errors::provider::ProviderResult;
use reth_taiko_primitives::L1Origin;

/// The trait for fetch L1 origin related data.
#[auto_impl::auto_impl(&, Arc)]
pub trait L1OriginReader: Send + Sync {
    /// Get the L1 origin for the given block hash.
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin>;
    /// Get the head L1 origin.
    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin>;
}

/// The trait for updating L1 origin related data.
#[auto_impl::auto_impl(&, Arc)]
pub trait L1OriginWriter: Send + Sync {
    /// Save the L1 origin for the given block hash.
    fn save_l1_origin(&self, block_number: BlockNumber, l1_origin: L1Origin) -> ProviderResult<()>;
}

/// The trait for providing taiko database operations.
pub trait TaikoProvider: L1OriginReader + L1OriginWriter {}

impl<T> TaikoProvider for T where T: L1OriginReader + L1OriginWriter {}
