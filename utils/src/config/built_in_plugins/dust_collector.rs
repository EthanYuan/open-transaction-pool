use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct DustCollectorConfig {
    pub enabled: bool,
    pub key: String,             // private key env name
    pub default_address: String, // default address env name
}
