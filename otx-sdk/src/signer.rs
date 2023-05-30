use otx_format::jsonrpc_types::OpenTransaction;

use anyhow::Result;
use ckb_sdk_otx::Address;
use ckb_types::H256;

use std::collections::HashMap;

#[derive(PartialEq)]
pub enum SighashMode {
    All = 0x01,
    None = 0x02,
    Single = 0x03,
    AllAnyoneCanPay = 0x81,
    NoneAnyoneCanPay = 0x82,
    SingleAnyoneCanPay = 0x83,
}

pub struct Signer {
    _accounts: HashMap<Address, H256>,
}

impl Signer {
    pub fn new(_accounts: Vec<(Address, H256)>) -> Self {
        todo!()
    }

    pub fn add_account(&mut self, _secp_address: Address, _private_key: H256) {
        todo!()
    }

    // This signing function will attempt to sign all inputs with existing private keys
    // (if they haven't been signed yet), and can only specify one mode for all inputs.
    pub fn partial_sign(
        &self,
        _otx: OpenTransaction,
        _mode: SighashMode,
    ) -> Result<OpenTransaction> {
        todo!()
    }

    pub fn all_sign(&self, _otx: OpenTransaction) -> Result<OpenTransaction> {
        todo!()
    }
}
