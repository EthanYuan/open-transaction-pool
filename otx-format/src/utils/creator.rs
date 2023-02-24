use crate::constant::extra_keys::OTX_VERSIONING_META_OPEN_TX_VERSION;
use crate::error::OtxFormatError;
use crate::jsonrpc_types::tx_view::tx_view_to_otx;
use crate::jsonrpc_types::{OpenTransaction, OtxKeyPair};

use anyhow::Result;
use ckb_jsonrpc_types::{JsonBytes, TransactionView, Uint32};
use ckb_sdk::{CkbRpcClient, IndexerRpcClient};
use ckb_types::prelude::{Entity, Pack};

pub struct Creator {
    _ckb_rpc_client: CkbRpcClient,
    _indexer_rpc_client: IndexerRpcClient,
}

impl Creator {
    pub fn new(uri: &str) -> Self {
        Creator {
            _ckb_rpc_client: CkbRpcClient::new(uri),
            _indexer_rpc_client: IndexerRpcClient::new(uri),
        }
    }

    pub fn new_from_tx_view(
        &self,
        tx_view: TransactionView,
    ) -> Result<OpenTransaction, OtxFormatError> {
        let mut otx = tx_view_to_otx(tx_view)?;
        add_extra_otx_version(&mut otx);
        add_extra_accounting(&mut otx);
        Ok(otx)
    }
}

fn add_extra_otx_version(otx: &mut OpenTransaction) {
    let otx_version = OtxKeyPair::new(
        OTX_VERSIONING_META_OPEN_TX_VERSION.into(),
        None,
        JsonBytes::from_bytes(Uint32::from(1u32).pack().as_bytes()),
    );
    append_key_to_meta(otx, otx_version);
}

fn add_extra_accounting(_otx: &mut OpenTransaction) {}

fn append_key_to_meta(otx: &mut OpenTransaction, key_pair: OtxKeyPair) {
    otx.meta.push(key_pair)
}
