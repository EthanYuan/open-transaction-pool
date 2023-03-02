use utils::const_definition::ScriptInfo;

use ansi_term::Colour::Green;
use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, Script};
use serde::{de::DeserializeOwned, Deserialize};

use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub network_config: NetworkConfig,
    pub scripts: Vec<ScriptConfig>,
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

pub fn print_logo() {
    println!(
        "{}",
        format_args!(
            r#"
            {}                   _____                               _   _             
            {}_ __   ___ _ __   |_   _| __ __ _ _ __  ___  __ _  ___| |_(_) ___  _ __  
            {} '_ \ / _ \ '_ \    | || '__/ _` | '_ \/ __|/ _` |/ __| __| |/ _ \| '_ \ 
            {} |_) |  __/ | | |   | || | | (_| | | | \__ \ (_| | (__| |_| | (_) | | | |
            {}| .__/ \___|_| |_|   |_||_|  \__,_|_| |_|___/\__,_|\___|\__|_|\___/|_| |_|
            {}|_|                                                                       
            "#,
            Green.bold().paint(r#"  ____  "#),
            Green.bold().paint(r#" / __ \ "#),
            Green.bold().paint(r#"| |  | |"#),
            Green.bold().paint(r#"| |__| |"#),
            Green.bold().paint(r#" \____/"#),
            Green.bold().paint(r#"       "#),
        )
    );
}
