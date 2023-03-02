#![allow(clippy::mutable_key_type)]

use super::{HeaderDep, OutputData, Witness};
use crate::constant::custom_keys::{
    OTX_ACCOUNTING_META_INPUT_CKB, OTX_ACCOUNTING_META_INPUT_XUDT, OTX_ACCOUNTING_META_OUTPUT_CKB,
    OTX_ACCOUNTING_META_OUTPUT_XUDT, OTX_IDENTIFYING_META_TX_HASH,
    OTX_IDENTIFYING_META_TX_WITNESS_HASH, OTX_LOCATING_INPUT_CAPACITY,
    OTX_VERSIONING_META_OPEN_TX_VERSION,
};
use crate::constant::essential_keys::OTX_META_VERSION;
use crate::error::OtxFormatError;
use crate::jsonrpc_types::{OpenTransaction, OtxKeyPair, OtxMap};

use anyhow::Result;
use ckb_jsonrpc_types::{
    CellDep, CellInput, CellOutput, JsonBytes, Script, TransactionView, Uint32,
};
use ckb_jsonrpc_types::{Uint128, Uint64};
use ckb_sdk::{CkbRpcClient, IndexerRpcClient};
use ckb_types::constants::TX_VERSION;
use ckb_types::core::TransactionBuilder;
use ckb_types::packed::{self, Transaction};
use ckb_types::prelude::*;
use ckb_types::H256;

use std::collections::HashMap;
use std::convert::Into;

pub fn tx_view_to_basic_otx(tx_view: TransactionView) -> Result<OpenTransaction, OtxFormatError> {
    let key_type: Uint32 = OTX_META_VERSION.into();
    let meta = vec![OtxKeyPair::new(
        key_type,
        None,
        JsonBytes::from_bytes(tx_view.inner.version.pack().as_bytes()),
    )];

    let cell_deps: Vec<OtxMap> = tx_view
        .inner
        .cell_deps
        .into_iter()
        .map(Into::into)
        .collect();

    let header_deps: Vec<OtxMap> = tx_view
        .inner
        .header_deps
        .into_iter()
        .map(Into::into)
        .collect();

    let inputs: Vec<OtxMap> = tx_view.inner.inputs.into_iter().map(Into::into).collect();

    let witnesses: Vec<OtxMap> = tx_view
        .inner
        .witnesses
        .into_iter()
        .map(Into::into)
        .collect();

    let outputs = tx_view
        .inner
        .outputs
        .into_iter()
        .zip(tx_view.inner.outputs_data.into_iter());
    let outputs: Vec<OtxMap> = outputs.map(Into::into).collect();

    Ok(OpenTransaction::new(
        meta.into(),
        cell_deps.into(),
        header_deps.into(),
        inputs.into(),
        witnesses.into(),
        outputs.into(),
    ))
}

pub fn tx_view_to_otx(
    tx_view: TransactionView,
    _min_ckb_fee: Option<u64>,
    _max_ckb_fee: Option<u64>,
    xudt_code_hash: H256,
    ckb_uri: &str,
) -> Result<OpenTransaction, OtxFormatError> {
    let mut ckb_rpc_client = CkbRpcClient::new(ckb_uri);
    let _indexer_rpc_client = IndexerRpcClient::new(ckb_uri);

    let mut input_ckb_capacity: u64 = 0;
    let mut output_ckb_capacity: u64 = 0;
    let mut xudt_input_map: HashMap<Script, u128> = HashMap::new();
    let mut xudt_output_map: HashMap<Script, u128> = HashMap::new();
    let core_tx_view = Transaction::from(tx_view.inner.clone()).into_view();

    let mut meta = vec![
        OtxKeyPair::new(
            OTX_META_VERSION.into(),
            None,
            JsonBytes::from_bytes(tx_view.inner.version.pack().as_bytes()),
        ),
        OtxKeyPair::new(
            OTX_VERSIONING_META_OPEN_TX_VERSION.into(),
            None,
            JsonBytes::from_bytes(Uint32::from(1).pack().as_bytes()),
        ),
        OtxKeyPair::new(
            OTX_IDENTIFYING_META_TX_HASH.into(),
            None,
            core_tx_view.hash().as_bytes().pack().into(),
        ),
        OtxKeyPair::new(
            OTX_IDENTIFYING_META_TX_WITNESS_HASH.into(),
            None,
            core_tx_view.witness_hash().as_bytes().pack().into(),
        ),
    ];

    let cell_deps: Vec<OtxMap> = tx_view
        .inner
        .cell_deps
        .into_iter()
        .map(Into::into)
        .collect();

    let header_deps: Vec<OtxMap> = tx_view
        .inner
        .header_deps
        .into_iter()
        .map(Into::into)
        .collect();

    let mut inputs = vec![];
    for input in tx_view.inner.inputs.into_iter() {
        let out_point = input.clone().previous_output;
        let mut otx_map: OtxMap = input.into();
        let cell_with_status = ckb_rpc_client
            .get_live_cell(out_point, true)
            .map_err(|err| OtxFormatError::LocateInputFailed(err.to_string()))?;
        if cell_with_status.cell.is_none() {
            return Err(OtxFormatError::LocateInputFailed(
                "does not exist".to_string(),
            ));
        }
        if cell_with_status.status != "live" {
            return Err(OtxFormatError::LocateInputFailed(cell_with_status.status));
        }
        let cell = cell_with_status.cell.unwrap();
        let input_capacity = OtxKeyPair::new(
            OTX_LOCATING_INPUT_CAPACITY.into(),
            Some(packed::Byte::default().as_bytes().pack().into()),
            JsonBytes::from_bytes(cell.output.capacity.pack().as_bytes()),
        );
        otx_map.push(input_capacity);
        input_ckb_capacity += <Uint64 as Into<u64>>::into(cell.output.capacity);
        inputs.push(otx_map);

        if let Some(type_) = cell.output.type_.clone() {
            if type_.code_hash == xudt_code_hash {
                if let Some(data) = cell.data {
                    if let Some(amount) = decode_udt_amount(data.content.as_bytes()) {
                        *xudt_input_map.entry(type_).or_insert(0) += amount;
                    }
                }
            }
        }
    }

    let witnesses: Vec<OtxMap> = tx_view
        .inner
        .witnesses
        .into_iter()
        .map(Into::into)
        .collect();

    let outputs_iter = tx_view
        .inner
        .outputs
        .into_iter()
        .zip(tx_view.inner.outputs_data.into_iter());
    let outputs: Vec<OtxMap> = outputs_iter
        .map(|output| {
            output_ckb_capacity += <Uint64 as Into<u64>>::into(output.0.capacity);
            if let Some(type_) = output.0.type_.clone() {
                if type_.code_hash == xudt_code_hash {
                    if let Some(amount) = decode_udt_amount(output.1.as_bytes()) {
                        *xudt_output_map.entry(type_).or_insert(0) += amount;
                    }
                }
            }
            output.into()
        })
        .collect();

    meta.push(OtxKeyPair::new(
        OTX_ACCOUNTING_META_INPUT_CKB.into(),
        None,
        JsonBytes::from_bytes(Uint64::from(input_ckb_capacity).pack().as_bytes()),
    ));
    meta.push(OtxKeyPair::new(
        OTX_ACCOUNTING_META_OUTPUT_CKB.into(),
        None,
        JsonBytes::from_bytes(Uint64::from(output_ckb_capacity).pack().as_bytes()),
    ));
    xudt_input_map
        .into_iter()
        .for_each(|(type_, input_xudt_amount)| {
            meta.push(OtxKeyPair::new(
                OTX_ACCOUNTING_META_INPUT_XUDT.into(),
                {
                    let script: packed::Script = type_.into();
                    Some(JsonBytes::from_bytes(script.as_bytes()))
                },
                JsonBytes::from_bytes(Uint128::from(input_xudt_amount).pack().as_bytes()),
            ));
        });
    xudt_output_map
        .into_iter()
        .for_each(|(type_, output_xudt_amount)| {
            meta.push(OtxKeyPair::new(
                OTX_ACCOUNTING_META_OUTPUT_XUDT.into(),
                {
                    let script: packed::Script = type_.into();
                    Some(JsonBytes::from_bytes(script.as_bytes()))
                },
                JsonBytes::from_bytes(Uint128::from(output_xudt_amount).pack().as_bytes()),
            ));
        });

    Ok(OpenTransaction::new(
        meta.into(),
        cell_deps.into(),
        header_deps.into(),
        inputs.into(),
        witnesses.into(),
        outputs.into(),
    ))
}

pub fn otx_to_tx_view(otx: OpenTransaction) -> Result<TransactionView, OtxFormatError> {
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

    let outputs: Vec<(CellOutput, OutputData)> =
        otx.outputs
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
    Ok(tx_view.into())
}

fn decode_udt_amount(data: &[u8]) -> Option<u128> {
    if data.len() < 16 {
        return None;
    }
    Some(u128::from_le_bytes(to_fixed_array(&data[0..16])))
}

fn to_fixed_array<const LEN: usize>(input: &[u8]) -> [u8; LEN] {
    assert_eq!(input.len(), LEN);
    let mut list = [0; LEN];
    list.copy_from_slice(input);
    list
}
