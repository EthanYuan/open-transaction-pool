use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct ScriptConfigItem {
    script_name: String,
    script: String,
    cell_dep: String,
}

impl ScriptConfigItem {
    pub fn new(script_name: &str, script: &str, cell_dep: &str) -> Self {
        ScriptConfigItem {
            script_name: script_name.to_string(),
            script: script.to_string(),
            cell_dep: cell_dep.to_string(),
        }
    }

    pub fn get_script_name(&self) -> &str {
        &self.script_name
    }

    pub fn get_script(&self) -> &str {
        &self.script
    }

    pub fn get_cell_dep(&self) -> &str {
        &self.cell_dep
    }
}
