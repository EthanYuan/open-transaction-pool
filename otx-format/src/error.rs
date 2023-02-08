use derive_more::Display;
use molecule::error::VerificationError;
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display};

pub trait OtxError: Debug + Display {
    fn err_code(&self) -> i64;
    fn message(&self) -> String;
}

#[derive(Serialize, Deserialize, Clone, Debug, Display, Hash, PartialEq, Eq)]
pub enum OtxFormatError {
    #[display(fmt = "version {} is not supported", _0)]
    VersionNotSupported(String),

    #[display(fmt = "{} map has duplicate keypairs", _0)]
    OtxMapHasDuplicateKeypair(String),

    #[display(fmt = "map parse missing field {}", _0)]
    OtxMapParseMissingField(String),

    #[display(fmt = "map parse failed: {}", _0)]
    OtxMapParseFailed(String),
}

impl OtxError for OtxFormatError {
    fn err_code(&self) -> i64 {
        match self {
            OtxFormatError::VersionNotSupported(_) => -13010,
            OtxFormatError::OtxMapHasDuplicateKeypair(_) => -13011,
            OtxFormatError::OtxMapParseMissingField(_) => -13012,
            OtxFormatError::OtxMapParseFailed(_) => -13013,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl OtxError for VerificationError {
    fn err_code(&self) -> i64 {
        match self {
            VerificationError::TotalSizeNotMatch(_, _, _) => -13000,
            VerificationError::HeaderIsBroken(_, _, _) => -13001,
            VerificationError::UnknownItem(_, _, _) => -13002,
            VerificationError::OffsetsNotMatch(_) => -13003,
            VerificationError::FieldCountNotMatch(_, _, _) => 13004,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}
