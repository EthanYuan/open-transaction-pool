use otx_format::jsonrpc_types::OpenTransaction;

use anyhow::Result;
use secp256k1::SecretKey;

pub(crate) const SIGHASH_ALL_SIGNATURE_SIZE: usize = 65;
pub(crate) const MAGIC_CODE: &str = "COTX";

#[derive(PartialEq)]
pub enum SighashMode {
    All = 0x01,
    None = 0x02,
    Single = 0x03,
    AllAnyoneCanPay = 0x81,
    NoneAnyoneCanPay = 0x82,
    SingleAnyoneCanPay = 0x83,
}

pub fn all_sign(_otx: OpenTransaction, _keys: Vec<SecretKey>) -> Result<OpenTransaction> {
    todo!()
}

pub fn partial_sign(
    _otx: OpenTransaction,
    _mode: SighashMode,
    _keys: Vec<SecretKey>,
) -> Result<OpenTransaction> {
    todo!()
}

pub fn partial_sign_input(
    _otx: OpenTransaction,
    _index: usize,
    _mode: SighashMode,
    _keys: Vec<SecretKey>,
) -> Result<OpenTransaction> {
    todo!()
}
