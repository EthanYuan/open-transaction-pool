pub mod built_in_plugins;
pub mod ckb_config;
pub mod network;
pub mod script;

pub use ckb_config::CkbConfig;
pub use network::NetworkConfig;
pub use script::ScriptConfigItem;

use built_in_plugins::{AtomicSwapConfig, DustCollectorConfig, P2PRelayerConfig, SignerConfig};

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, DepType, Script};
use ckb_types::packed;
use ckb_types::prelude::*;
use ckb_types::H256;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use std::collections::HashMap;
use std::{fs::File, io::Read, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInfo {
    pub script: packed::Script,
    pub cell_dep: packed::CellDep,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ConfigFile {
    pub network_config: NetworkConfig,
    pub ckb_config: CkbConfig,
    pub scripts: Vec<ScriptConfigItem>,
    pub built_in_plugin_dust_collector: DustCollectorConfig,
    pub built_in_plugin_atomic_swap: AtomicSwapConfig,
    pub built_in_plugin_signer: SignerConfig,
    pub built_in_plugin_p2p_relayer: P2PRelayerConfig,
}

impl ConfigFile {
    pub fn to_script_map(&self) -> HashMap<String, ScriptInfo> {
        self.scripts
            .iter()
            .map(|s| {
                (
                    s.get_script_name().to_owned(),
                    ScriptInfo {
                        script: serde_json::from_str::<Script>(s.get_script())
                            .expect("config string to script")
                            .into(),
                        cell_dep: serde_json::from_str::<CellDep>(s.get_cell_dep())
                            .expect("config string to cell dep")
                            .into(),
                    },
                )
            })
            .collect()
    }
}

impl From<ConfigFile> for AppConfig {
    fn from(config_file: ConfigFile) -> Self {
        Self::new(config_file)
    }
}

pub struct AppConfig {
    network_config: NetworkConfig,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    plugin_dust_collector_config: DustCollectorConfig,
    plugin_atomic_swap_config: AtomicSwapConfig,
    plugin_signer_config: SignerConfig,
    plugin_p2p_relayer_config: P2PRelayerConfig,
}

impl AppConfig {
    pub fn new(config_file: ConfigFile) -> Self {
        let script_config = config_file.to_script_map();
        Self {
            network_config: config_file.network_config,
            ckb_config: config_file.ckb_config,
            script_config: ScriptConfig::new(script_config),
            plugin_dust_collector_config: config_file.built_in_plugin_dust_collector,
            plugin_atomic_swap_config: config_file.built_in_plugin_atomic_swap,
            plugin_signer_config: config_file.built_in_plugin_signer,
            plugin_p2p_relayer_config: config_file.built_in_plugin_p2p_relayer,
        }
    }

    pub fn get_ckb_config(&self) -> CkbConfig {
        self.ckb_config.clone()
    }

    pub fn get_network_config(&self) -> NetworkConfig {
        self.network_config.clone()
    }

    pub fn get_script_config(&self) -> ScriptConfig {
        self.script_config.clone()
    }

    pub fn get_dust_collector_config(&self) -> DustCollectorConfig {
        self.plugin_dust_collector_config.clone()
    }

    pub fn get_atomic_swap_config(&self) -> AtomicSwapConfig {
        self.plugin_atomic_swap_config.clone()
    }

    pub fn get_signer_config(&self) -> SignerConfig {
        self.plugin_signer_config.clone()
    }

    pub fn get_p2p_relayer_config(&self) -> P2PRelayerConfig {
        self.plugin_p2p_relayer_config.clone()
    }
}

#[derive(Clone, Debug)]
pub struct ScriptConfig {
    script_map: HashMap<String, ScriptInfo>,
}

impl ScriptConfig {
    pub fn new(script_map: HashMap<String, ScriptInfo>) -> Self {
        Self { script_map }
    }

    pub fn get_script_info(&self, script_name: &str) -> Option<ScriptInfo> {
        self.script_map.get(script_name).cloned()
    }

    pub fn get_cell_dep(&self, script_name: &str) -> Option<CellDep> {
        self.script_map
            .get(script_name)
            .map(|s| s.cell_dep.clone().into())
    }

    pub fn get_secp256k1_blake160_sighash_all_code_hash(&self) -> H256 {
        self.script_map
            .get("secp256k1_blake160")
            .expect("secp256k1_blake160")
            .script
            .code_hash()
            .unpack()
    }

    pub fn get_xudt_rce_code_hash(&self) -> H256 {
        self.script_map
            .get("xudt_rce")
            .expect("xudt_rce")
            .script
            .code_hash()
            .unpack()
    }

    pub fn get_omni_lock_code_hash(&self) -> H256 {
        self.script_map
            .get("omni_lock")
            .expect("omni_lock")
            .script
            .code_hash()
            .unpack()
    }

    pub fn get_anyone_can_pay_code_hash(&self) -> H256 {
        self.script_map
            .get("anyone_can_pay")
            .expect("anyone_can_pay")
            .script
            .code_hash()
            .unpack()
    }

    pub fn get_sudt_code_hash(&self) -> H256 {
        self.script_map
            .get("sudt")
            .expect("sudt")
            .script
            .code_hash()
            .unpack()
    }

    pub fn get_secp_data_cell_dep(&self) -> CellDep {
        let tx_hash = self
            .script_map
            .get("dao")
            .expect("dao")
            .cell_dep
            .out_point()
            .tx_hash();
        let out_point = ckb_types::packed::OutPoint::new_builder()
            .tx_hash(tx_hash)
            .index(3u32.pack())
            .build();
        CellDep {
            out_point: out_point.into(),
            dep_type: DepType::Code,
        }
    }

    pub fn get_xdut_cell_dep(&self) -> CellDep {
        self.script_map
            .get("xudt_rce")
            .expect("xudt_rce")
            .cell_dep
            .clone()
            .into()
    }

    pub fn get_omni_lock_cell_dep(&self) -> CellDep {
        self.script_map
            .get("omni_lock")
            .expect("omni_lock")
            .cell_dep
            .clone()
            .into()
    }
}

pub fn parse<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    parse_reader(&mut File::open(name)?)
}

fn parse_reader<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf)?;
    Ok(toml::from_slice(&buf)?)
}
