use utils::aggregator::SignInfo;

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, Script};
use serde::{de::DeserializeOwned, Deserialize};

use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub network_config: NetworkConfig,
    pub scripts: Vec<ScriptConfig>,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct NetworkConfig {
    pub network_type: String,
    pub ckb_uri: String,
    pub listen_uri: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ScriptConfig {
    pub script_name: String,
    pub script: String,
    pub cell_dep: String,
}

pub fn parse<T: DeserializeOwned>(name: impl AsRef<Path>) -> Result<T> {
    parse_reader(&mut File::open(name)?)
}

fn parse_reader<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf)?;
    Ok(toml::from_slice(&buf)?)
}
