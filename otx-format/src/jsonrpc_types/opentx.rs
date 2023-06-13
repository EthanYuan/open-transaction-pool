#![allow(clippy::mutable_key_type)]

use crate::constant::extra_keys::{
    OTX_ACCOUNTING_META_INPUT_CKB, OTX_ACCOUNTING_META_INPUT_SUDT, OTX_ACCOUNTING_META_INPUT_XUDT,
    OTX_ACCOUNTING_META_MAX_FEE, OTX_ACCOUNTING_META_OUTPUT_CKB, OTX_ACCOUNTING_META_OUTPUT_SUDT,
    OTX_ACCOUNTING_META_OUTPUT_XUDT, OTX_IDENTIFYING_META_AGGREGATE_COUNT,
    OTX_IDENTIFYING_META_TX_HASH,
};
use crate::error::OtxFormatError;
use crate::jsonrpc_types::otx_map::{OtxKeyPair, OtxMap};
use crate::types::packed::{self, OpenTransactionBuilder, OtxMapVecBuilder};
use crate::types::PaymentAmount;

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, CellInput, CellOutput, JsonBytes, TransactionView};
use ckb_types::constants::TX_VERSION;
use ckb_types::core::{self, TransactionBuilder};
use ckb_types::packed::{Uint128, Uint64};
use ckb_types::{self, prelude::*, H256};
use serde::{Deserialize, Serialize};

pub type HeaderDep = H256;
pub type Witness = JsonBytes;
pub type OutputData = JsonBytes;

use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct OtxMapVec(Vec<OtxMap>);

impl IntoIterator for OtxMapVec {
    type Item = OtxMap;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<OtxMap>> for OtxMapVec {
    fn from(vec: Vec<OtxMap>) -> Self {
        OtxMapVec(vec)
    }
}

impl From<OtxMapVec> for packed::OtxMapVec {
    fn from(json: OtxMapVec) -> Self {
        let map_vec: Vec<packed::OtxMap> = json.0.into_iter().map(Into::into).collect();
        OtxMapVecBuilder::default().set(map_vec).build()
    }
}

impl From<packed::OtxMapVec> for OtxMapVec {
    fn from(packed: packed::OtxMapVec) -> Self {
        OtxMapVec(packed.into_iter().map(Into::into).collect())
    }
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct OpenTransaction {
    pub meta: OtxMap,
    pub cell_deps: OtxMapVec,
    pub header_deps: OtxMapVec,
    pub inputs: OtxMapVec,
    pub witnesses: OtxMapVec,
    pub outputs: OtxMapVec,
}

impl OpenTransaction {
    pub fn new(
        meta: OtxMap,
        cell_deps: OtxMapVec,
        header_deps: OtxMapVec,
        inputs: OtxMapVec,
        witnesses: OtxMapVec,
        outputs: OtxMapVec,
    ) -> Self {
        OpenTransaction {
            meta,
            cell_deps,
            header_deps,
            inputs,
            witnesses,
            outputs,
        }
    }

    pub fn get_or_insert_otx_id(&mut self) -> Result<H256, OtxFormatError> {
        if let Some(value_data) = self.meta.get(OTX_IDENTIFYING_META_TX_HASH.into(), None) {
            H256::from_slice(value_data.as_bytes())
                .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))
        } else {
            let id = self.get_tx_hash()?;
            self.meta.push(OtxKeyPair::new(
                OTX_IDENTIFYING_META_TX_HASH.into(),
                None,
                JsonBytes::from_bytes(id.as_bytes().to_owned().into()),
            ));
            Ok(id)
        }
    }

    pub fn get_tx_hash(&self) -> Result<H256, OtxFormatError> {
        let tx_view: Result<TransactionView, _> = self.to_owned().try_into();
        tx_view.map(|tx| tx.hash)
    }

    pub fn get_tx_witness_hash(&self) -> Result<H256, OtxFormatError> {
        let tx_view: Result<core::TransactionView, _> = self.to_owned().try_into();
        tx_view.map(|tx| tx.witness_hash().unpack())
    }

    pub fn get_aggregate_count(&self) -> Result<u32, OtxFormatError> {
        let aggregate_count = self
            .meta
            .get(OTX_IDENTIFYING_META_AGGREGATE_COUNT.into(), None)
            .map(|aggregate_count| {
                let count: u32 = ckb_types::packed::Uint32::from_slice(aggregate_count.as_bytes())
                    .expect("get aggregate count")
                    .unpack();
                count
            })
            .ok_or_else(|| {
                OtxFormatError::OtxMapParseMissingField(
                    OTX_IDENTIFYING_META_AGGREGATE_COUNT.to_string(),
                )
            })?;
        Ok(aggregate_count)
    }

    pub fn get_max_fee(&self) -> u64 {
        self.meta
            .get(OTX_ACCOUNTING_META_MAX_FEE.into(), None)
            .map(|max_fee| {
                let count: u64 = ckb_types::packed::Uint64::from_slice(max_fee.as_bytes())
                    .expect("get aggregate count")
                    .unpack();
                count
            })
            .unwrap_or(0)
    }

    pub fn get_payment_amount(&self) -> Result<PaymentAmount, OtxFormatError> {
        // capacity
        let input_capacity = self
            .meta
            .get(OTX_ACCOUNTING_META_INPUT_CKB.into(), None)
            .map(|input_ckb| {
                let capacity: u64 = Uint64::from_slice(input_ckb.as_bytes())
                    .expect("get input ckb")
                    .unpack();
                capacity
            })
            .ok_or_else(|| {
                OtxFormatError::OtxMapParseMissingField(OTX_ACCOUNTING_META_INPUT_CKB.to_string())
            })?;

        let output_capacity = self
            .meta
            .get(OTX_ACCOUNTING_META_OUTPUT_CKB.into(), None)
            .map(|output_ckb| {
                let capacity: u64 = Uint64::from_slice(output_ckb.as_bytes())
                    .expect("get output ckb")
                    .unpack();
                capacity
            })
            .ok_or_else(|| {
                OtxFormatError::OtxMapParseMissingField(OTX_ACCOUNTING_META_OUTPUT_CKB.to_string())
            })?;

        // fee
        let fee = self.get_max_fee();

        let mut kv_map = self.meta.clone();
        let mut x_udt_amount = HashMap::new();
        loop {
            let input_xudt_amount =
                kv_map.pop_entry_by_first_element(OTX_ACCOUNTING_META_INPUT_XUDT.into());
            if input_xudt_amount.is_none() {
                break;
            }
            let ((_, script), input_xudt_amount) = input_xudt_amount.unwrap();
            let script = ckb_types::packed::Script::from_slice(
                script.to_owned().expect("get script").as_bytes(),
            )
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .into();
            let input_xudt_amount: u128 = Uint128::from_slice(input_xudt_amount.as_bytes())
                .expect("get input xudt amount")
                .unpack();
            *x_udt_amount.entry(script).or_insert(0) += input_xudt_amount as i128;
        }
        loop {
            let output_xudt_amount =
                kv_map.pop_entry_by_first_element(OTX_ACCOUNTING_META_OUTPUT_XUDT.into());
            if output_xudt_amount.is_none() {
                break;
            }
            let ((_, script), output_xudt_amount) = output_xudt_amount.unwrap();
            let script = ckb_types::packed::Script::from_slice(
                script.to_owned().expect("get script").as_bytes(),
            )
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .into();
            let output_xudt_amount: u128 = Uint128::from_slice(output_xudt_amount.as_bytes())
                .expect("get output xudt amount")
                .unpack();
            *x_udt_amount.entry(script).or_insert(0) -= output_xudt_amount as i128;
        }

        let mut s_udt_amount = HashMap::new();
        loop {
            let input_sudt_amount =
                kv_map.pop_entry_by_first_element(OTX_ACCOUNTING_META_INPUT_SUDT.into());
            if input_sudt_amount.is_none() {
                break;
            }
            let ((_, script), input_sudt_amount) = input_sudt_amount.unwrap();
            let script = ckb_types::packed::Script::from_slice(
                script.to_owned().expect("get script").as_bytes(),
            )
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .into();
            let input_sudt_amount: u128 = Uint128::from_slice(input_sudt_amount.as_bytes())
                .expect("get input sudt amount")
                .unpack();
            *s_udt_amount.entry(script).or_insert(0) += input_sudt_amount as i128;
        }
        loop {
            let output_sudt_amount =
                kv_map.pop_entry_by_first_element(OTX_ACCOUNTING_META_OUTPUT_SUDT.into());
            if output_sudt_amount.is_none() {
                break;
            }
            let ((_, script), output_sudt_amount) = output_sudt_amount.unwrap();
            let script = ckb_types::packed::Script::from_slice(
                script.to_owned().expect("get script").as_bytes(),
            )
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .into();
            let output_sudt_amount: u128 = Uint128::from_slice(output_sudt_amount.as_bytes())
                .expect("get output sudt amount")
                .unpack();
            *s_udt_amount.entry(script).or_insert(0) -= output_sudt_amount as i128;
        }

        Ok(PaymentAmount {
            capacity: input_capacity as i128 - output_capacity as i128,
            fee,
            x_udt_amount,
            s_udt_amount,
        })
    }
}

impl From<OpenTransaction> for packed::OpenTransaction {
    fn from(json: OpenTransaction) -> Self {
        OpenTransactionBuilder::default()
            .meta(json.meta.into())
            .cell_deps(json.cell_deps.into())
            .header_deps(json.header_deps.into())
            .inputs(json.inputs.into())
            .witnesses(json.witnesses.into())
            .outputs(json.outputs.into())
            .build()
    }
}

impl From<packed::OpenTransaction> for OpenTransaction {
    fn from(packed: packed::OpenTransaction) -> Self {
        OpenTransaction {
            meta: packed.meta().into(),
            cell_deps: packed.cell_deps().into(),
            header_deps: packed.header_deps().into(),
            inputs: packed.inputs().into(),
            witnesses: packed.witnesses().into(),
            outputs: packed.outputs().into(),
        }
    }
}

impl TryFrom<OpenTransaction> for core::TransactionView {
    type Error = OtxFormatError;
    fn try_from(otx: OpenTransaction) -> Result<Self, Self::Error> {
        let witnesses = otx
            .witnesses
            .into_iter()
            .map(|witness| witness.try_into())
            .collect::<Result<Vec<Witness>, _>>()?;

        let inputs = otx
            .inputs
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<CellInput>, _>>()?;

        let outputs: Vec<(CellOutput, OutputData)> = otx
            .outputs
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<(CellOutput, OutputData)>, _>>()?;
        let (outputs, outputs_data): (Vec<_>, Vec<_>) =
            outputs.into_iter().map(|(a, b)| (a, b)).unzip();

        let cell_deps = otx
            .cell_deps
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<CellDep>, _>>()?;

        let header_deps = otx
            .header_deps
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<HeaderDep>, _>>()?;

        let tx_view = TransactionBuilder::default()
            .version(TX_VERSION.pack())
            .witnesses(
                witnesses
                    .into_iter()
                    .map(|witness| witness.as_bytes().pack()),
            )
            .inputs(inputs.into_iter().map(Into::into))
            .outputs(outputs.into_iter().map(Into::into))
            .outputs_data(outputs_data.into_iter().map(Into::into))
            .cell_deps(cell_deps.into_iter().map(Into::into))
            .header_deps(header_deps.into_iter().map(|h| h.pack()))
            .build();
        Ok(tx_view)
    }
}

impl TryFrom<OpenTransaction> for TransactionView {
    type Error = OtxFormatError;
    fn try_from(otx: OpenTransaction) -> Result<Self, Self::Error> {
        TryInto::<core::TransactionView>::try_into(otx).map(Into::into)
    }
}
