use ckb_types::H256;
use otx_format::error::{OtxError, OtxFormatError};

use anyhow::Result;
use derive_more::Display;
use jsonrpc_core::{Error, ErrorCode};
use molecule::error::VerificationError;
use serde::{Deserialize, Serialize};

use std::fmt::Debug;

pub type InnerResult<T> = Result<T, OtxRpcError>;

#[derive(Debug, Display)]
pub struct OtxRpcError(pub Box<dyn OtxError + Send>);

impl From<OtxRpcError> for Error {
    fn from(err: OtxRpcError) -> Error {
        Error {
            code: ErrorCode::ServerError(err.0.err_code()),
            message: err.0.message(),
            data: None,
        }
    }
}

impl From<OtxFormatError> for OtxRpcError {
    fn from(err: OtxFormatError) -> Self {
        OtxRpcError(Box::new(err))
    }
}

impl From<VerificationError> for OtxRpcError {
    fn from(err: VerificationError) -> Self {
        OtxRpcError(Box::new(err))
    }
}

impl From<OtxPoolError> for OtxRpcError {
    fn from(err: OtxPoolError) -> Self {
        OtxRpcError(Box::new(err))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum OtxPoolError {
    #[display(fmt = "Otx already exists")]
    OtxAlreadyExists,

    #[display(fmt = "RPC parameter parsing error: {}", _0)]
    RpcParamParseError(String),

    #[display(fmt = "RPC parameter parsing error: {}", _0)]
    TxNotFound(H256),
}

impl OtxError for OtxPoolError {
    fn err_code(&self) -> i64 {
        match self {
            OtxPoolError::OtxAlreadyExists => -13100,
            OtxPoolError::RpcParamParseError(_) => -13101,
            OtxPoolError::TxNotFound(_) => -13102,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}
