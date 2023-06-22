#![allow(clippy::mutable_key_type)]

use crate::constant::essential_keys::{
    OTX_CELL_DEP_OUTPOINT_INDEX, OTX_CELL_DEP_OUTPOINT_TX_HASH, OTX_CELL_DEP_TYPE,
    OTX_HEADER_DEP_HASH, OTX_INPUT_OUTPOINT_INDEX, OTX_INPUT_OUTPOINT_TX_HASH, OTX_INPUT_SINCE,
    OTX_OUTPUT_CAPACITY, OTX_OUTPUT_DATA, OTX_OUTPUT_LOCK_ARGS, OTX_OUTPUT_LOCK_CODE_HASH,
    OTX_OUTPUT_LOCK_HASH_TYPE, OTX_OUTPUT_TYPE_ARGS, OTX_OUTPUT_TYPE_CODE_HASH,
    OTX_OUTPUT_TYPE_HASH_TYPE, OTX_WITNESS_RAW,
};

use crate::error::OtxFormatError;
use crate::types::packed::{self, OtxMapBuilder};

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, CellInput, CellOutput, DepType, JsonBytes, Script, Uint32};
use ckb_types::bytes::Bytes;
use ckb_types::core::{self, ScriptHashType};
use ckb_types::packed::{Byte32, OutPointBuilder, WitnessArgs};
use ckb_types::{self, prelude::*, H256};
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

pub type HeaderDep = H256;
pub type Witness = JsonBytes;
pub type OutputData = JsonBytes;

use std::convert::TryFrom;
use std::hash::Hash;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
struct OtxKey {
    key_type: Uint32,
    key_data: Option<JsonBytes>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
struct OtxValue(JsonBytes);

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

impl From<OtxKeyPair> for (OtxKey, OtxValue) {
    fn from(pair: OtxKeyPair) -> Self {
        let key = OtxKey {
            key_type: pair.key_type,
            key_data: pair.key_data,
        };
        let value = OtxValue(pair.value_data);
        (key, value)
    }
}

impl From<(OtxKey, OtxValue)> for OtxKeyPair {
    fn from(pair: (OtxKey, OtxValue)) -> Self {
        let key_type = pair.0.key_type;
        let key_data = pair.0.key_data;
        let value_data = pair.1 .0;
        OtxKeyPair {
            key_type,
            key_data,
            value_data,
        }
    }
}

impl From<(&OtxKey, &OtxValue)> for OtxKeyPair {
    fn from(pair: (&OtxKey, &OtxValue)) -> Self {
        let key_type = pair.0.key_type;
        let key_data = pair.0.key_data.clone();
        let value_data = pair.1 .0.clone();
        OtxKeyPair {
            key_type,
            key_data,
            value_data,
        }
    }
}

impl From<(OtxKey, OtxValue)> for packed::OtxKeyPair {
    fn from(pair: (OtxKey, OtxValue)) -> Self {
        let key_type = pair.0.key_type.pack();
        let key_data = pair.0.key_data.map(|data| data.into_bytes()).pack();
        let value_data = pair.1 .0.into_bytes().pack();
        packed::OtxKeyPairBuilder::default()
            .key_type(key_type)
            .key_data(key_data)
            .value_data(value_data)
            .build()
    }
}

impl From<OtxKeyPair> for packed::OtxKeyPair {
    fn from(pair: OtxKeyPair) -> Self {
        packed::OtxKeyPairBuilder::default()
            .key_type(pair.key_type.pack())
            .key_data(pair.key_data.map(|data| data.into_bytes()).pack())
            .value_data(pair.value_data.into_bytes().pack())
            .build()
    }
}

impl From<packed::OtxKeyPair> for (OtxKey, OtxValue) {
    fn from(packed: packed::OtxKeyPair) -> Self {
        let key_type = packed.key_type().unpack();
        let key_data = packed.key_data().to_opt().map(Into::into);
        let value_data = packed.value_data().into();
        (OtxKey { key_type, key_data }, OtxValue(value_data))
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct OtxMap(LinkedHashMap<OtxKey, OtxValue>);

impl Serialize for OtxMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.iter().collect::<Vec<_>>().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OtxMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<(OtxKey, OtxValue)>::deserialize(deserializer)?;
        let map = vec.into_iter().collect::<LinkedHashMap<_, _>>();
        Ok(OtxMap(map))
    }
}

impl IntoIterator for OtxMap {
    type Item = OtxKeyPair;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl OtxMap {
    pub fn new() -> Self {
        OtxMap(LinkedHashMap::new())
    }

    pub fn push(&mut self, key_pair: OtxKeyPair) {
        let pair: (OtxKey, OtxValue) = key_pair.into();
        self.0.insert(pair.0, pair.1);
    }

    pub fn get(&self, key_type: Uint32, key_data: Option<JsonBytes>) -> Option<JsonBytes> {
        self.0
            .get(&OtxKey { key_type, key_data })
            .map(|value| value.0.clone())
    }

    pub fn pop_entry_by_first_element(
        &mut self,
        first_element: Uint32,
    ) -> Option<((Uint32, Option<JsonBytes>), JsonBytes)> {
        let found_key = self
            .0
            .keys()
            .find(|otx_key| otx_key.key_type == first_element)?
            .to_owned();
        let value = self.0.remove(&found_key)?;
        Some(((found_key.key_type, found_key.key_data), value.0))
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

impl From<Vec<OtxKeyPair>> for OtxMap {
    fn from(vec: Vec<OtxKeyPair>) -> Self {
        let map = vec
            .into_iter()
            .map(Into::into)
            .collect::<LinkedHashMap<_, _>>();
        OtxMap(map)
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
    fn try_from(mut map: OtxMap) -> Result<Self, Self::Error> {
        let out_point_tx_hash = map
            .0
            .remove(&OtxKey {
                key_type: OTX_CELL_DEP_OUTPOINT_TX_HASH.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue(Byte32::zero().as_bytes().pack().into()));
        let out_point_index = map
            .0
            .remove(&OtxKey {
                key_type: OTX_CELL_DEP_OUTPOINT_INDEX.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue({
                let value: ckb_types::packed::Uint32 = 0xffffffffu32.pack();
                value.as_bytes().pack().into()
            }));
        let out_point = OutPointBuilder::default()
            .tx_hash(
                ckb_types::packed::Byte32::from_slice(out_point_tx_hash.0.as_bytes()).map_err(
                    |e| {
                        OtxFormatError::OtxMapParseFailed(
                            OTX_CELL_DEP_OUTPOINT_TX_HASH,
                            e.to_string(),
                        )
                    },
                )?,
            )
            .index(
                ckb_types::packed::Uint32::from_slice(out_point_index.0.as_bytes()).map_err(
                    |e| {
                        OtxFormatError::OtxMapParseFailed(
                            OTX_CELL_DEP_OUTPOINT_INDEX,
                            e.to_string(),
                        )
                    },
                )?,
            )
            .build()
            .into();

        let dep_type = map
            .0
            .remove(&OtxKey {
                key_type: OTX_CELL_DEP_TYPE.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue(packed::Byte::default().as_bytes().pack().into()));
        let dep_type: ckb_types::core::DepType = packed::Byte::from_slice(dep_type.0.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(OTX_CELL_DEP_TYPE, e.to_string()))?
            .try_into()
            .map_err(|_| {
                OtxFormatError::OtxMapParseFailed(OTX_CELL_DEP_TYPE, "CellDep".to_string())
            })?;
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
    fn try_from(mut map: OtxMap) -> Result<Self, Self::Error> {
        let header_dep = map
            .0
            .remove(&OtxKey {
                key_type: OTX_HEADER_DEP_HASH.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue(Byte32::zero().as_bytes().pack().into()));
        let header_dep = HeaderDep::from_slice(header_dep.0.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(OTX_HEADER_DEP_HASH, e.to_string()))?;

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
    fn try_from(mut map: OtxMap) -> Result<Self, Self::Error> {
        let witness = map
            .0
            .remove(&OtxKey {
                key_type: OTX_WITNESS_RAW.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue(WitnessArgs::default().as_bytes().pack().into()))
            .0;
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
    fn try_from(mut map: OtxMap) -> Result<Self, Self::Error> {
        let out_point_tx_hash = map
            .0
            .remove(&OtxKey {
                key_type: OTX_INPUT_OUTPOINT_TX_HASH.into(),
                key_data: None,
            })
            .unwrap_or(OtxValue(Byte32::zero().as_bytes().pack().into()));
        let out_point_index = map
            .0
            .remove(&OtxKey {
                key_type: OTX_INPUT_OUTPOINT_INDEX.into(),
                key_data: None,
            })
            .unwrap_or({
                let value: ckb_types::packed::Uint32 = 0xffffffffu32.pack();
                OtxValue(value.as_bytes().pack().into())
            });
        let previous_output = OutPointBuilder::default()
            .tx_hash(
                ckb_types::packed::Byte32::from_slice(out_point_tx_hash.0.as_bytes()).map_err(
                    |e| {
                        OtxFormatError::OtxMapParseFailed(OTX_INPUT_OUTPOINT_TX_HASH, e.to_string())
                    },
                )?,
            )
            .index(
                ckb_types::packed::Uint32::from_slice(out_point_index.0.as_bytes()).map_err(
                    |e| OtxFormatError::OtxMapParseFailed(OTX_INPUT_OUTPOINT_INDEX, e.to_string()),
                )?,
            )
            .build()
            .into();

        let since = map
            .0
            .remove(&OtxKey {
                key_type: OTX_INPUT_SINCE.into(),
                key_data: None,
            })
            .unwrap_or_else(|| {
                let value: ckb_types::packed::Uint64 = 0u64.pack();
                OtxValue(value.as_bytes().pack().into())
            });
        let since = ckb_types::packed::Uint64::from_slice(since.0.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(OTX_INPUT_SINCE, e.to_string()))?
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
    fn try_from(mut map: OtxMap) -> Result<Self, Self::Error> {
        // capacity
        let capacity = map
            .0
            .remove(&OtxKey {
                key_type: OTX_OUTPUT_CAPACITY.into(),
                key_data: None,
            })
            .unwrap_or_else(|| {
                let value: ckb_types::packed::Uint64 = 0u64.pack();
                OtxValue(value.as_bytes().pack().into())
            });

        let capacity = ckb_types::packed::Uint64::from_slice(capacity.0.as_bytes())
            .map_err(|e| OtxFormatError::OtxMapParseFailed(OTX_OUTPUT_CAPACITY, e.to_string()))?
            .unpack();

        // lock code hash
        let lock_code_hash = map
            .0
            .remove(&OtxKey {
                key_type: OTX_OUTPUT_LOCK_CODE_HASH.into(),
                key_data: None,
            })
            .unwrap_or_else(|| OtxValue(Byte32::zero().as_bytes().pack().into()));
        let lock_code_hash = ckb_types::packed::Byte32::from_slice(lock_code_hash.0.as_bytes())
            .map_err(|e| {
                OtxFormatError::OtxMapParseFailed(OTX_OUTPUT_LOCK_CODE_HASH, e.to_string())
            })?
            .unpack();

        // lock hash type
        let lock_hash_type = map
            .0
            .remove(&OtxKey {
                key_type: OTX_OUTPUT_LOCK_HASH_TYPE.into(),
                key_data: None,
            })
            .unwrap_or_else(|| OtxValue(packed::Byte::default().as_bytes().pack().into()));
        let lock_hash_type: u8 = packed::Byte::from_slice(lock_hash_type.0.as_bytes())
            .map_err(|e| {
                OtxFormatError::OtxMapParseFailed(OTX_OUTPUT_LOCK_HASH_TYPE, e.to_string())
            })?
            .into();
        let lock_hash_type: ScriptHashType = lock_hash_type.try_into().map_err(|_| {
            OtxFormatError::OtxMapParseFailed(
                OTX_OUTPUT_LOCK_HASH_TYPE,
                "ScriptHashType".to_string(),
            )
        })?;

        // lock args
        let lock_args = map
            .0
            .remove(&OtxKey {
                key_type: OTX_OUTPUT_LOCK_ARGS.into(),
                key_data: None,
            })
            .unwrap_or_else(|| OtxValue(Bytes::new().pack().into()))
            .0;

        let type_ = if map
            .0
            .get(&OtxKey {
                key_type: OTX_OUTPUT_TYPE_CODE_HASH.into(),
                key_data: None,
            })
            .is_none()
            && map
                .0
                .get(&OtxKey {
                    key_type: OTX_OUTPUT_TYPE_HASH_TYPE.into(),
                    key_data: None,
                })
                .is_none()
            && map
                .0
                .get(&OtxKey {
                    key_type: OTX_OUTPUT_TYPE_ARGS.into(),
                    key_data: None,
                })
                .is_none()
        {
            None
        } else {
            let type_code_hash = map
                .0
                .remove(&OtxKey {
                    key_type: OTX_OUTPUT_TYPE_CODE_HASH.into(),
                    key_data: None,
                })
                .unwrap_or_else(|| OtxValue(Byte32::zero().as_bytes().pack().into()));
            let type_code_hash = ckb_types::packed::Byte32::from_slice(type_code_hash.0.as_bytes())
                .map_err(|e| {
                    OtxFormatError::OtxMapParseFailed(OTX_OUTPUT_TYPE_CODE_HASH, e.to_string())
                })?
                .unpack();

            let type_hash_type = map
                .0
                .remove(&OtxKey {
                    key_type: OTX_OUTPUT_TYPE_HASH_TYPE.into(),
                    key_data: None,
                })
                .unwrap_or_else(|| OtxValue(packed::Byte::default().as_bytes().pack().into()));
            let type_hash_type: u8 = packed::Byte::from_slice(type_hash_type.0.as_bytes())
                .map_err(|e| {
                    OtxFormatError::OtxMapParseFailed(OTX_OUTPUT_TYPE_HASH_TYPE, e.to_string())
                })?
                .into();
            let type_hash_type: ScriptHashType = type_hash_type.try_into().map_err(|_| {
                OtxFormatError::OtxMapParseFailed(
                    OTX_OUTPUT_TYPE_HASH_TYPE,
                    "ScriptHashType".to_string(),
                )
            })?;

            let type_args = map
                .0
                .remove(&OtxKey {
                    key_type: OTX_OUTPUT_TYPE_ARGS.into(),
                    key_data: None,
                })
                .unwrap_or_else(|| OtxValue(Bytes::new().pack().into()))
                .0;

            Some(Script {
                code_hash: type_code_hash,
                hash_type: type_hash_type.into(),
                args: type_args,
            })
        };

        // output data
        let output_data = map
            .0
            .remove(&OtxKey {
                key_type: OTX_OUTPUT_DATA.into(),
                key_data: None,
            })
            .unwrap_or_else(|| OtxValue(Bytes::new().pack().into()))
            .0;

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
