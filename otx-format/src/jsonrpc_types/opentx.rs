#![allow(clippy::mutable_key_type)]

use crate::constant::essential_keys::{
    OTX_CELL_DEP_OUTPOINT_INDEX, OTX_CELL_DEP_OUTPOINT_TX_HASH, OTX_CELL_DEP_TYPE,
    OTX_HEADER_DEP_HASH, OTX_INPUT_OUTPOINT_INDEX, OTX_INPUT_OUTPOINT_TX_HASH, OTX_INPUT_SINCE,
    OTX_OUTPUT_CAPACITY, OTX_OUTPUT_DATA, OTX_OUTPUT_LOCK_ARGS, OTX_OUTPUT_LOCK_CODE_HASH,
    OTX_OUTPUT_LOCK_HASH_TYPE, OTX_OUTPUT_TYPE_ARGS, OTX_OUTPUT_TYPE_CODE_HASH,
    OTX_OUTPUT_TYPE_HASH_TYPE, OTX_WITNESS_RAW,
};
use crate::constant::extra_keys::{
    OTX_ACCOUNTING_META_INPUT_CKB, OTX_ACCOUNTING_META_INPUT_SUDT, OTX_ACCOUNTING_META_INPUT_XUDT,
    OTX_ACCOUNTING_META_OUTPUT_CKB, OTX_ACCOUNTING_META_OUTPUT_SUDT,
    OTX_ACCOUNTING_META_OUTPUT_XUDT, OTX_IDENTIFYING_META_TX_HASH,
};
use crate::error::OtxFormatError;
use crate::types::packed::{self, OpenTransactionBuilder, OtxMapBuilder, OtxMapVecBuilder};

use anyhow::Result;
use ckb_jsonrpc_types::{
    CellDep, CellInput, CellOutput, DepType, JsonBytes, Script, TransactionView, Uint32,
};
use ckb_types::bytes::Bytes;
use ckb_types::constants::TX_VERSION;
use ckb_types::core::{self, ScriptHashType, TransactionBuilder};
use ckb_types::packed::{Byte32, OutPointBuilder, Uint128, Uint64, WitnessArgs};
use ckb_types::{self, prelude::*, H256};
use serde::{Deserialize, Serialize};

pub type HeaderDep = H256;
pub type Witness = JsonBytes;
pub type OutputData = JsonBytes;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::hash::Hash;
use std::slice::Iter;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct OtxKeyPair {
    key_type: Uint32,
    key_data: Option<JsonBytes>,
    value_data: JsonBytes,
}

impl OtxKeyPair {
    pub fn new(key_type: Uint32, key_data: Option<JsonBytes>, value_data: JsonBytes) -> Self {
        OtxKeyPair {
            key_type,
            key_data,
            value_data,
        }
    }
}

impl From<OtxKeyPair> for packed::OtxKeyPair {
    fn from(json: OtxKeyPair) -> Self {
        packed::OtxKeyPairBuilder::default()
            .key_type(json.key_type.pack())
            .key_data(json.key_data.map(|data| data.into_bytes()).pack())
            .value_data(json.value_data.into_bytes().pack())
            .build()
    }
}

impl From<packed::OtxKeyPair> for OtxKeyPair {
    fn from(packed: packed::OtxKeyPair) -> Self {
        OtxKeyPair {
            key_type: packed.key_type().unpack(),
            key_data: packed.key_data().to_opt().map(Into::into),
            value_data: packed.value_data().into(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
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

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
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
        if let Some(key_pair) = self
            .meta
            .iter()
            .find(|key_pair| key_pair.key_type == OTX_IDENTIFYING_META_TX_HASH.into())
        {
            H256::from_slice(key_pair.value_data.as_bytes())
                .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))
        } else {
            let id = self.get_tx_hash()?;
            self.meta.push(OtxKeyPair {
                key_type: OTX_IDENTIFYING_META_TX_HASH.into(),
                key_data: None,
                value_data: JsonBytes::from_bytes(id.as_bytes().to_owned().into()),
            });
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

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct OtxMap(Vec<OtxKeyPair>);

impl IntoIterator for OtxMap {
    type Item = OtxKeyPair;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl OtxMap {
    pub fn iter(&self) -> Iter<OtxKeyPair> {
        self.0.iter()
    }

    pub fn push_unique(&mut self, key_pair: OtxKeyPair) -> Result<(), OtxFormatError> {
        // check duplicate key
        if self.0.iter().any(|k| k.key_type == key_pair.key_type) {
            Err(OtxFormatError::OtxMapHasDuplicateKeypair(
                key_pair.key_type.to_string(),
            ))
        } else {
            self.0.push(key_pair);
            Ok(())
        }
    }

    pub fn push(&mut self, key_pair: OtxKeyPair) {
        self.0.push(key_pair)
    }
}

impl From<Vec<OtxKeyPair>> for OtxMap {
    fn from(vec: Vec<OtxKeyPair>) -> Self {
        OtxMap(vec)
    }
}

impl From<OtxMap> for packed::OtxMap {
    fn from(json: OtxMap) -> Self {
        let map: Vec<packed::OtxKeyPair> = json.0.into_iter().map(Into::into).collect();
        OtxMapBuilder::default().set(map).build()
    }
}

impl From<packed::OtxMap> for OtxMap {
    fn from(packed: packed::OtxMap) -> Self {
        OtxMap(packed.into_iter().map(Into::into).collect())
    }
}

impl From<CellDep> for OtxMap {
    fn from(cell_dep: CellDep) -> Self {
        let out_point: ckb_types::packed::OutPoint = cell_dep.out_point.into();
        let out_point_tx_hash = OtxKeyPair::new(
            OTX_CELL_DEP_OUTPOINT_TX_HASH.into(),
            None,
            JsonBytes::from_bytes(out_point.tx_hash().as_bytes()),
        );
        let out_point_index = OtxKeyPair::new(
            OTX_CELL_DEP_OUTPOINT_INDEX.into(),
            None,
            JsonBytes::from_bytes(out_point.index().as_bytes()),
        );
        let dep_type: core::DepType = cell_dep.dep_type.into();
        let dep_type: ckb_types::packed::Byte = dep_type.into();
        let dep_type = OtxKeyPair::new(
            OTX_CELL_DEP_TYPE.into(),
            None,
            JsonBytes::from_bytes(dep_type.as_bytes()),
        );
        vec![out_point_tx_hash, out_point_index, dep_type].into()
    }
}

impl TryFrom<OtxMap> for CellDep {
    type Error = OtxFormatError;
    fn try_from(map: OtxMap) -> Result<Self, Self::Error> {
        let mut kv_map = to_kv_map(&map)?;

        let out_point_tx_hash = kv_map
            .remove(&OTX_CELL_DEP_OUTPOINT_TX_HASH)
            .unwrap_or((None, Byte32::zero().as_bytes().pack().into()));
        let out_point_index = kv_map
            .remove(&OTX_CELL_DEP_OUTPOINT_INDEX)
            .unwrap_or_else(|| {
                let value: ckb_types::packed::Uint32 = 0xffffffffu32.pack();
                (None, value.as_bytes().pack().into())
            });
        let out_point = OutPointBuilder::default()
            .tx_hash(
                ckb_types::packed::Byte32::from_slice(out_point_tx_hash.1.as_bytes())
                    .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?,
            )
            .index(
                ckb_types::packed::Uint32::from_slice(out_point_index.1.as_bytes())
                    .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?,
            )
            .build()
            .into();

        let dep_type = kv_map
            .remove(&OTX_CELL_DEP_TYPE)
            .unwrap_or((None, packed::Byte::default().as_bytes().pack().into()));
        let dep_type: ckb_types::core::DepType = packed::Byte::from_slice(dep_type.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .try_into()
            .map_err(|_| OtxFormatError::OtxMapParseFailed("CellDep".to_string()))?;
        let dep_type: DepType = dep_type.into();

        Ok(CellDep {
            out_point,
            dep_type,
        })
    }
}

impl From<HeaderDep> for OtxMap {
    fn from(header_dep: HeaderDep) -> Self {
        let header_dep = OtxKeyPair::new(
            OTX_HEADER_DEP_HASH.into(),
            None,
            JsonBytes::from_bytes(header_dep.pack().as_bytes()),
        );
        vec![header_dep].into()
    }
}

impl TryFrom<OtxMap> for HeaderDep {
    type Error = OtxFormatError;
    fn try_from(map: OtxMap) -> Result<Self, Self::Error> {
        let mut kv_map = to_kv_map(&map)?;

        let header_dep = kv_map
            .remove(&OTX_HEADER_DEP_HASH)
            .unwrap_or((None, Byte32::zero().as_bytes().pack().into()));
        let header_dep = HeaderDep::from_slice(header_dep.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?;

        Ok(header_dep)
    }
}

impl From<Witness> for OtxMap {
    fn from(witness: Witness) -> Self {
        let witness = OtxKeyPair::new(OTX_WITNESS_RAW.into(), None, witness);
        vec![witness].into()
    }
}

impl TryFrom<OtxMap> for Witness {
    type Error = OtxFormatError;
    fn try_from(map: OtxMap) -> Result<Self, Self::Error> {
        let mut kv_map = to_kv_map(&map)?;
        let witness = kv_map
            .remove(&OTX_WITNESS_RAW)
            .unwrap_or((None, WitnessArgs::default().as_bytes().pack().into()))
            .1;
        Ok(witness)
    }
}

impl From<CellInput> for OtxMap {
    fn from(cell_input: CellInput) -> Self {
        let previous_output: ckb_types::packed::OutPoint = cell_input.previous_output.into();
        let out_point_tx_hash = OtxKeyPair::new(
            OTX_INPUT_OUTPOINT_TX_HASH.into(),
            None,
            JsonBytes::from_bytes(previous_output.tx_hash().as_bytes()),
        );
        let out_point_index = OtxKeyPair::new(
            OTX_INPUT_OUTPOINT_INDEX.into(),
            None,
            JsonBytes::from_bytes(previous_output.index().as_bytes()),
        );

        let since = cell_input.since.pack();
        let since = OtxKeyPair::new(
            OTX_INPUT_SINCE.into(),
            None,
            JsonBytes::from_bytes(since.as_bytes()),
        );

        vec![out_point_tx_hash, out_point_index, since].into()
    }
}

impl TryFrom<OtxMap> for CellInput {
    type Error = OtxFormatError;
    fn try_from(map: OtxMap) -> Result<Self, Self::Error> {
        let mut kv_map = to_kv_map(&map)?;

        let out_point_tx_hash = kv_map
            .remove(&OTX_INPUT_OUTPOINT_TX_HASH)
            .unwrap_or((None, Byte32::zero().as_bytes().pack().into()));
        let out_point_index = kv_map.remove(&OTX_INPUT_OUTPOINT_INDEX).unwrap_or_else(|| {
            let value: ckb_types::packed::Uint32 = 0xffffffffu32.pack();
            (None, value.as_bytes().pack().into())
        });
        let previous_output = OutPointBuilder::default()
            .tx_hash(
                ckb_types::packed::Byte32::from_slice(out_point_tx_hash.1.as_bytes())
                    .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?,
            )
            .index(
                ckb_types::packed::Uint32::from_slice(out_point_index.1.as_bytes())
                    .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?,
            )
            .build()
            .into();

        let since = kv_map.remove(&OTX_INPUT_SINCE).unwrap_or_else(|| {
            let value: ckb_types::packed::Uint64 = 0u64.pack();
            (None, value.as_bytes().pack().into())
        });
        let since = ckb_types::packed::Uint64::from_slice(since.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .unpack();

        Ok(CellInput {
            since,
            previous_output,
        })
    }
}

impl From<(CellOutput, OutputData)> for OtxMap {
    fn from(output: (CellOutput, OutputData)) -> Self {
        let capacity = OtxKeyPair::new(
            OTX_OUTPUT_CAPACITY.into(),
            None,
            JsonBytes::from_bytes(output.0.capacity.pack().as_bytes()),
        );
        let lock_code_hash = OtxKeyPair::new(
            OTX_OUTPUT_LOCK_CODE_HASH.into(),
            None,
            JsonBytes::from_bytes(output.0.lock.code_hash.pack().as_bytes()),
        );
        let lock_hash_type: ScriptHashType = output.0.lock.hash_type.into();
        let lock_hash_type: packed::Byte = lock_hash_type.into();
        let lock_hash_type = OtxKeyPair::new(
            OTX_OUTPUT_LOCK_HASH_TYPE.into(),
            None,
            JsonBytes::from_bytes(lock_hash_type.as_bytes()),
        );
        let lock_args = OtxKeyPair::new(OTX_OUTPUT_LOCK_ARGS.into(), None, output.0.lock.args);
        let mut map = vec![capacity, lock_code_hash, lock_hash_type, lock_args];

        if let Some(type_) = output.0.type_ {
            let type_code_hash = OtxKeyPair::new(
                OTX_OUTPUT_TYPE_CODE_HASH.into(),
                None,
                JsonBytes::from_bytes(type_.code_hash.pack().as_bytes()),
            );
            map.push(type_code_hash);
            let type_hash_type: ScriptHashType = type_.hash_type.into();
            let type_hash_type: packed::Byte = type_hash_type.into();
            let type_hash_type = OtxKeyPair::new(
                OTX_OUTPUT_TYPE_HASH_TYPE.into(),
                None,
                JsonBytes::from_bytes(type_hash_type.as_bytes()),
            );
            map.push(type_hash_type);
            let type_args = OtxKeyPair::new(OTX_OUTPUT_TYPE_ARGS.into(), None, type_.args);
            map.push(type_args);
        };

        let data = OtxKeyPair::new(OTX_OUTPUT_DATA.into(), None, output.1);
        map.push(data);

        map.into()
    }
}

impl TryFrom<OtxMap> for (CellOutput, OutputData) {
    type Error = OtxFormatError;
    fn try_from(map: OtxMap) -> Result<Self, Self::Error> {
        let mut kv_map = to_kv_map(&map)?;

        // capacity
        let capacity = kv_map.remove(&OTX_OUTPUT_CAPACITY).unwrap_or_else(|| {
            let value: ckb_types::packed::Uint64 = 0u64.pack();
            (None, value.as_bytes().pack().into())
        });
        let capacity = ckb_types::packed::Uint64::from_slice(capacity.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .unpack();

        // lock code hash
        let lock_code_hash = kv_map
            .remove(&OTX_OUTPUT_LOCK_CODE_HASH)
            .unwrap_or((None, Byte32::zero().as_bytes().pack().into()));
        let lock_code_hash = ckb_types::packed::Byte32::from_slice(lock_code_hash.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .unpack();

        // lock hash type
        let lock_hash_type = kv_map
            .remove(&OTX_OUTPUT_LOCK_HASH_TYPE)
            .unwrap_or((None, packed::Byte::default().as_bytes().pack().into()));
        let lock_hash_type: u8 = packed::Byte::from_slice(lock_hash_type.1.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
            .into();
        let lock_hash_type: ScriptHashType = lock_hash_type
            .try_into()
            .map_err(|_| OtxFormatError::OtxMapParseFailed("ScriptHashType".to_string()))?;

        // lock args
        let lock_args = kv_map
            .remove(&OTX_OUTPUT_LOCK_ARGS)
            .unwrap_or((None, Bytes::new().pack().into()))
            .1;

        let type_ = if kv_map.get(&OTX_OUTPUT_TYPE_CODE_HASH).is_none()
            && kv_map.get(&OTX_OUTPUT_TYPE_HASH_TYPE).is_none()
            && kv_map.get(&OTX_OUTPUT_TYPE_ARGS).is_none()
        {
            None
        } else {
            let type_code_hash = kv_map
                .remove(&OTX_OUTPUT_TYPE_CODE_HASH)
                .unwrap_or((None, Byte32::zero().as_bytes().pack().into()));
            let type_code_hash = ckb_types::packed::Byte32::from_slice(type_code_hash.1.as_bytes())
                .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
                .unpack();

            let type_hash_type = kv_map
                .remove(&OTX_OUTPUT_TYPE_HASH_TYPE)
                .unwrap_or((None, packed::Byte::default().as_bytes().pack().into()));
            let type_hash_type: u8 = packed::Byte::from_slice(type_hash_type.1.as_bytes())
                .map_err(|e| OtxFormatError::OtxMapParseFailed(e.to_string()))?
                .into();
            let type_hash_type: ScriptHashType = type_hash_type
                .try_into()
                .map_err(|_| OtxFormatError::OtxMapParseFailed("ScriptHashType".to_string()))?;

            let type_args = kv_map
                .remove(&OTX_OUTPUT_TYPE_ARGS)
                .unwrap_or((None, Bytes::new().pack().into()))
                .1;

            Some(Script {
                code_hash: type_code_hash,
                hash_type: type_hash_type.into(),
                args: type_args,
            })
        };

        // output data
        let output_data = kv_map
            .remove(&OTX_OUTPUT_DATA)
            .unwrap_or((None, Bytes::new().pack().into()))
            .1;

        let cell_output = CellOutput {
            capacity,
            lock: Script {
                code_hash: lock_code_hash,
                hash_type: lock_hash_type.into(),
                args: lock_args,
            },
            type_,
        };

        Ok((cell_output, output_data))
    }
}

fn to_kv_map(
    iter: &OtxMap,
) -> Result<HashMap<u32, (Option<JsonBytes>, JsonBytes)>, OtxFormatError> {
    let mut map = HashMap::new();
    for pair in iter.iter() {
        if map
            .insert(
                pair.key_type.value(),
                (pair.key_data.to_owned(), pair.value_data.to_owned()),
            )
            .is_some()
        {
            return Err(OtxFormatError::OtxMapHasDuplicateKeypair(
                pair.key_type.to_string(),
            ));
        }
    }
    Ok(map)
}

fn to_tuple_kv_map(
    iter: &OtxMap,
) -> Result<HashMap<(u32, Option<JsonBytes>), JsonBytes>, OtxFormatError> {
    let mut map = HashMap::new();
    for pair in iter.iter() {
        if map
            .insert(
                (pair.key_type.value(), pair.key_data.to_owned()),
                pair.value_data.to_owned(),
            )
            .is_some()
        {
            return Err(OtxFormatError::OtxMapHasDuplicateKeypair(
                pair.key_type.to_string(),
            ));
        }
    }
    Ok(map)
}

#[derive(Debug)]
pub struct PaymentAmount {
    pub capacity: i128,
    pub x_udt_amount: HashMap<Script, i128>,
    pub s_udt_amount: HashMap<Script, i128>,
}

pub fn get_payment_amount(otx: &OpenTransaction) -> Result<PaymentAmount, OtxFormatError> {
    fn get_value_by_first_element(
        map: &HashMap<(u32, Option<JsonBytes>), JsonBytes>,
        first_element: u32,
    ) -> Option<&JsonBytes> {
        let found_key = map.keys().find(|(element, _)| *element == first_element);
        found_key.and_then(|key| map.get(key))
    }
    fn pop_entry_by_first_element(
        map: &mut HashMap<(u32, Option<JsonBytes>), JsonBytes>,
        first_element: u32,
    ) -> Option<((u32, Option<JsonBytes>), JsonBytes)> {
        let found_key = map
            .keys()
            .find(|(element, _)| *element == first_element)?
            .to_owned();
        let value = map.remove(&found_key)?;
        Some((found_key, value))
    }

    let mut kv_map = to_tuple_kv_map(&otx.meta)?;

    // capacity
    let input_capacity = get_value_by_first_element(&kv_map, OTX_ACCOUNTING_META_INPUT_CKB)
        .map(|input_ckb| {
            let capacity: u64 = Uint64::from_slice(input_ckb.as_bytes())
                .expect("get input ckb")
                .unpack();
            capacity
        })
        .ok_or_else(|| {
            OtxFormatError::OtxMapParseMissingField(OTX_ACCOUNTING_META_INPUT_CKB.to_string())
        })?;

    let output_capacity = get_value_by_first_element(&kv_map, OTX_ACCOUNTING_META_OUTPUT_CKB)
        .map(|output_ckb| {
            let capacity: u64 = Uint64::from_slice(output_ckb.as_bytes())
                .expect("get output ckb")
                .unpack();
            capacity
        })
        .ok_or_else(|| {
            OtxFormatError::OtxMapParseMissingField(OTX_ACCOUNTING_META_OUTPUT_CKB.to_string())
        })?;

    let mut x_udt_amount = HashMap::new();
    loop {
        let input_xudt_amount =
            pop_entry_by_first_element(&mut kv_map, OTX_ACCOUNTING_META_INPUT_XUDT);
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
            pop_entry_by_first_element(&mut kv_map, OTX_ACCOUNTING_META_OUTPUT_XUDT);
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
            .expect("get input xudt amount")
            .unpack();
        *x_udt_amount.entry(script).or_insert(0) -= output_xudt_amount as i128;
    }

    let mut s_udt_amount = HashMap::new();
    loop {
        let input_sudt_amount =
            pop_entry_by_first_element(&mut kv_map, OTX_ACCOUNTING_META_INPUT_SUDT);
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
            .expect("get input xudt amount")
            .unpack();
        *s_udt_amount.entry(script).or_insert(0) += input_sudt_amount as i128;
    }
    loop {
        let output_sudt_amount =
            pop_entry_by_first_element(&mut kv_map, OTX_ACCOUNTING_META_OUTPUT_SUDT);
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
            .expect("get input xudt amount")
            .unpack();
        *s_udt_amount.entry(script).or_insert(0) -= output_sudt_amount as i128;
    }

    Ok(PaymentAmount {
        capacity: input_capacity as i128 - output_capacity as i128,
        x_udt_amount,
        s_udt_amount,
    })
}
