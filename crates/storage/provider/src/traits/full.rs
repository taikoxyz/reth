//! Helper provider traits to encapsulate all provider traits for simplicity.

use crate::{
    AccountReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, ChangeSetReader,
    DatabaseProviderFactory, EvmEnvProvider, L1OriginReader, StageCheckpointReader,
    StateProviderFactory, StaticFileProviderFactory,
};
use reth_db_api::database::Database;

/// Helper trait to unify all provider traits for simplicity.
pub trait FullProvider<DB: Database>:
    DatabaseProviderFactory<DB>
    + StaticFileProviderFactory
    + BlockReaderIdExt
    + AccountReader
    + StateProviderFactory
    + EvmEnvProvider
    + ChainSpecProvider
    + ChangeSetReader
    + CanonStateSubscriptions
    + StageCheckpointReader
    + L1OriginReader
    + Clone
    + Unpin
    + 'static
{
}

impl<T, DB: Database> FullProvider<DB> for T where
    T: DatabaseProviderFactory<DB>
        + StaticFileProviderFactory
        + BlockReaderIdExt
        + AccountReader
        + StateProviderFactory
        + EvmEnvProvider
        + ChainSpecProvider
        + ChangeSetReader
        + CanonStateSubscriptions
        + StageCheckpointReader
        + L1OriginReader
        + Clone
        + Unpin
        + 'static
{
}
