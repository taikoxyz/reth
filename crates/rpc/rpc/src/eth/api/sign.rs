//! Contains RPC handler implementations specific to sign endpoints

use crate::{
    eth::{
        error::{EthResult, SignError},
        signer::{DevSigner, EthSigner},
    },
    EthApi,
};
use alloy_dyn_abi::TypedData;
use reth_primitives::{Address, Bytes};

impl<Provider, Pool, Network, EvmConfig> EthApi<Provider, Pool, Network, EvmConfig> {
    pub(crate) async fn sign(&self, account: Address, message: &[u8]) -> EthResult<Bytes> {
        Ok(self.find_signer(&account)?.sign(account, message).await?.to_hex_bytes())
    }

    pub(crate) fn sign_typed_data(&self, data: &TypedData, account: Address) -> EthResult<Bytes> {
        Ok(self.find_signer(&account)?.sign_typed_data(account, data)?.to_hex_bytes())
    }

    pub(crate) fn find_signer(
        &self,
        account: &Address,
    ) -> Result<Box<(dyn EthSigner + 'static)>, SignError> {
        self.inner
            .signers
            .read()
            .iter()
            .find(|signer| signer.is_signer_for(account))
            .map(|signer| dyn_clone::clone_box(&**signer))
            .ok_or(SignError::NoAccount)
    }

    /// Generates 20 random developer accounts.
    /// Used in DEV mode.
    pub fn with_dev_accounts(&self) {
        let mut signers = self.inner.signers.write();
        *signers = DevSigner::random_signers(20);
    }
}
