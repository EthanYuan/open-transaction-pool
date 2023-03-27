use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ScriptConfig {
    pub script_name: String,
    pub script: String,
    pub cell_dep: String,
}
