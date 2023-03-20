use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct DustCollectorConfig {
    pub enabled: bool,
    pub key: String,
    pub default_address: String,
}
