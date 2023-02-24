use super::{HeaderDep, OutputData, Witness};
use crate::constant::basic_keys::OTX_META_VERSION;
use crate::constant::extra_keys::{
    OTX_ACCOUNTING_META_INPUT_CKB, OTX_ACCOUNTING_META_OUTPUT_CKB, OTX_LOCATING_INPUT_CAPACITY,
    OTX_VERSIONING_META_OPEN_TX_VERSION,
};
use crate::error::OtxFormatError;
use crate::jsonrpc_types::{OpenTransaction, OtxKeyPair, OtxMap};
use crate::types::packed;

use anyhow::Result;
use ckb_jsonrpc_types::Uint64;
use ckb_jsonrpc_types::{CellDep, CellInput, CellOutput, JsonBytes, TransactionView, Uint32};
use ckb_sdk::{CkbRpcClient, IndexerRpcClient};
use ckb_types::constants::TX_VERSION;
use ckb_types::core::TransactionBuilder;
use ckb_types::prelude::{Entity, Pack};

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
    ckb_uri: &str,
) -> Result<OpenTransaction, OtxFormatError> {
    let mut ckb_rpc_client = CkbRpcClient::new(ckb_uri);
    let _indexer_rpc_client = IndexerRpcClient::new(ckb_uri);

    let mut input_ckb_capacity: u64 = 0;
    let mut output_ckb_capacity: u64 = 0;
    let mut _input_xudt_amount = 0;
    let mut _output_xudt_amount = 0;
    let mut _input_sudt_amount = 0;
    let mut _output_sudt_amount = 0;
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
