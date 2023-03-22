pub mod built_in_plugins;
pub mod network;
pub mod script;

pub use network::NetworkConfig;
pub use script::ScriptConfig;

use built_in_plugins::{AtomicSwapConfig, DustCollectorConfig};
use utils::const_definition::ScriptInfo;

use ckb_jsonrpc_types::{CellDep, Script};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub network_config: NetworkConfig,
    pub scripts: Vec<ScriptConfig>,
    pub built_in_plugin_dust_collector: DustCollectorConfig,
    pub built_in_plugin_atomic_swap: AtomicSwapConfig,
}

impl Config {
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
