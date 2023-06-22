use otx_pool_config::CkbConfig;

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::TransactionView;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH, traits::DefaultTransactionDependencyProvider,
    traits::SecpCkbRawKeySigner, tx_builder::unlock_tx, unlock::ScriptUnlocker,
    unlock::SecpSighashUnlocker, Address, ScriptGroup, ScriptId,
};
use ckb_types::{core::TransactionView as CoreTransactionView, packed::Transaction, H256};

use std::collections::HashMap;

#[derive(Clone)]
pub struct SignInfo {
    secp_address: Address,
    pk: H256,
    ckb_config: CkbConfig,
}

impl SignInfo {
    pub fn new(secp_address: &Address, pk: &H256, ckb_config: CkbConfig) -> Self {
        SignInfo {
            secp_address: secp_address.clone(),
            pk: pk.clone(),
            ckb_config,
        }
    }

    pub fn secp_address(&self) -> &Address {
        &self.secp_address
    }

    pub fn privkey(&self) -> &H256 {
        &self.pk
    }

    pub fn sign_ckb_tx(&self, tx_view: TransactionView) -> Result<TransactionView> {
        let tx = Transaction::from(tx_view.inner).into_view();
        let (tx, _) = self.sighash_sign(&[self.pk.clone()], tx)?;
        Ok(TransactionView::from(tx))
    }

    pub fn sighash_sign(
        &self,
        keys: &[H256],
        tx: CoreTransactionView,
    ) -> Result<(CoreTransactionView, Vec<ScriptGroup>)> {
        if keys.is_empty() {
            return Err(anyhow!("must provide sender-key to sign"));
        }
        let secret_keys = keys
            .iter()
            .map(|key| secp256k1::SecretKey::from_slice(key.as_bytes()))
            .collect::<Result<Vec<secp256k1::SecretKey>, _>>()?;

        // Build ScriptUnlocker
        let signer = SecpCkbRawKeySigner::new_with_secret_keys(secret_keys);
        let sighash_unlocker = SecpSighashUnlocker::from(Box::new(signer) as Box<_>);
        let sighash_script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
        let mut unlockers = HashMap::default();
        unlockers.insert(
            sighash_script_id,
            Box::new(sighash_unlocker) as Box<dyn ScriptUnlocker>,
        );

        let tx_dep_provider =
            DefaultTransactionDependencyProvider::new(self.ckb_config.get_ckb_uri(), 10);
        let (new_tx, new_still_locked_groups) = unlock_tx(tx, &tx_dep_provider, &unlockers)?;
        Ok((new_tx, new_still_locked_groups))
    }
}
