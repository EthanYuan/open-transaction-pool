use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct DustCollectorConfig {
    enabled: bool,
    key: String,             // private key env name
    default_address: String, // default address env name
}

impl DustCollectorConfig {
    pub fn new(enabled: bool, key: &str, default_address: &str) -> Self {
        DustCollectorConfig {
            enabled,
            key: key.to_string(),
            default_address: default_address.to_string(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_env_key_name(&self) -> &str {
        &self.key
    }

    pub fn get_env_default_address(&self) -> &str {
        &self.default_address
    }
}
