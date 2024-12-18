//! The module for L1 origin related data.
use alloy_primitives::BlockNumber;
use reth_db::tables::{HeadL1Origin, L1Origins};
use reth_db_api::transaction::{DbTx, DbTxMut};
use reth_node_types::NodeTypes;
use reth_provider::{
    providers::{BlockchainProvider, ProviderNodeTypes},
    DatabaseProvider, DatabaseProviderFactory, ProviderError,
};
use reth_storage_api::DBProvider;
use reth_storage_errors::provider::ProviderResult;
use reth_taiko_primitives::{HeadL1OriginKey, L1Origin};

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

impl<TX: DbTx + 'static, N: NodeTypes> L1OriginReader for DatabaseProvider<TX, N> {
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin> {
        self.tx_ref()
            .get::<L1Origins>(block_number)?
            .ok_or(ProviderError::L1OriginNotFound(block_number))
    }

    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin> {
        let block_number = self
            .tx_ref()
            .get::<HeadL1Origin>(HeadL1OriginKey)?
            .ok_or(ProviderError::HeadL1OriginNotFound)?;
        self.get_l1_origin(block_number)
    }
}

impl<TX: DbTxMut + DbTx + 'static, N: NodeTypes> L1OriginWriter for DatabaseProvider<TX, N> {
    fn save_l1_origin(&self, block_number: BlockNumber, l1_origin: L1Origin) -> ProviderResult<()> {
        self.tx_ref().put::<L1Origins>(block_number, l1_origin)?;
        self.tx_ref().put::<HeadL1Origin>(HeadL1OriginKey, block_number)?;
        Ok(())
    }
}

impl<N: ProviderNodeTypes> L1OriginReader for BlockchainProvider<N> {
    fn get_l1_origin(&self, block_number: BlockNumber) -> ProviderResult<L1Origin> {
        self.database_provider_ro()?.get_l1_origin(block_number)
    }

    fn get_head_l1_origin(&self) -> ProviderResult<L1Origin> {
        self.database_provider_ro()?.get_head_l1_origin()
    }
}

impl<N: ProviderNodeTypes> L1OriginWriter for BlockchainProvider<N> {
    fn save_l1_origin(&self, block_number: BlockNumber, l1_origin: L1Origin) -> ProviderResult<()> {
        let provider_rw = self.database_provider_rw()?;
        provider_rw.save_l1_origin(block_number, l1_origin)?;
        provider_rw.commit()?;
        Ok(())
    }
}
