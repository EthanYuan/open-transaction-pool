pub mod built_in_plugins;
pub mod ckb_config;
pub mod network;
pub mod script;

use crate::const_definition::ScriptInfo;

pub use ckb_config::CkbConfig;
pub use network::NetworkConfig;
pub use script::ScriptConfig;

use built_in_plugins::{AtomicSwapConfig, DustCollectorConfig};

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, Script};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use std::collections::HashMap;
use std::{fs::File, io::Read, path::Path};

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ConfigFile {
    pub network_config: NetworkConfig,
    pub ckb_config: CkbConfig,
    pub scripts: Vec<ScriptConfig>,
    pub built_in_plugin_dust_collector: DustCollectorConfig,
    pub built_in_plugin_atomic_swap: AtomicSwapConfig,
}

impl ConfigFile {
    pub fn to_script_map(&self) -> HashMap<String, ScriptInfo> {
        self.scripts
            .iter()
            .map(|s| {
                (
                    s.script_name.clone(),
                    ScriptInfo {
                        script: serde_json::from_str::<Script>(&s.script)
                            .expect("config string to script")
                            .into(),
                        cell_dep: serde_json::from_str::<CellDep>(&s.cell_dep)
                            .expect("config string to cell dep")
                            .into(),
                    },
                )
            })
            .collect()
    }
}

pub struct AppConfig {
    pub network_config: NetworkConfig,
    pub ckb_config: CkbConfig,
    pub script_map: HashMap<String, ScriptInfo>,
    pub built_in_plugin_dust_collector: DustCollectorConfig,
    pub built_in_plugin_atomic_swap: AtomicSwapConfig,
}

impl AppConfig {
    pub fn new(config_file: ConfigFile) -> Self {
        let script_map = config_file.to_script_map();
        Self {
            network_config: config_file.network_config,
            ckb_config: config_file.ckb_config,
            script_map,
            built_in_plugin_dust_collector: config_file.built_in_plugin_dust_collector,
            built_in_plugin_atomic_swap: config_file.built_in_plugin_atomic_swap,
        }
    }

    pub fn get_script_info(&self, script_name: &str) -> Option<ScriptInfo> {
        self.script_map.get(script_name).cloned()
    }

    pub fn get_script(&self, script_name: &str) -> Option<Script> {
        self.script_map
            .get(script_name)
            .map(|s| s.script.clone().into())
    }

    pub fn get_cell_dep(&self, script_name: &str) -> Option<CellDep> {
        self.script_map
            .get(script_name)
            .map(|s| s.cell_dep.clone().into())
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
