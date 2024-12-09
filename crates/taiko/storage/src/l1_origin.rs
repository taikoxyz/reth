use alloy_primitives::BlockNumber;
use reth_db_api::transaction::DbTx;
use reth_db_api::Database;
use reth_node_types::NodeTypesWithDB;
use reth_provider::providers::BlockchainProvider;
use reth_provider::DatabaseProvider;
use reth_storage_errors::provider::ProviderResult;
use taiko_reth_primitives::L1Origin;

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

impl<TX: DbTx, Spec: Send + Sync> L1OriginReader for DatabaseProvider<TX, Spec> {
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin> {
        todo!()
    }

    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin> {
        todo!()
    }
}

impl<TX: DbTx, Spec: Send + Sync> L1OriginWriter for DatabaseProvider<TX, Spec> {
    fn save_l1_origin(&self, block_number: BlockNumber, l1_origin: L1Origin) -> ProviderResult<()> {
        todo!()
    }
}

impl<DB: NodeTypesWithDB> L1OriginReader for BlockchainProvider<DB>
where
    DB: Database,
{
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin> {
        todo!()
    }

    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin> {
        todo!()
    }
}

impl<DB: NodeTypesWithDB> L1OriginWriter for BlockchainProvider<DB>
where
    DB: Database,
{
    fn save_l1_origin(&self, block_number: BlockNumber, l1_origin: L1Origin) -> ProviderResult<()> {
        todo!()
    }
}
