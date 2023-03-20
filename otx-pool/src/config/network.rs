use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct NetworkConfig {
    pub network_type: String,
    pub ckb_uri: String,
    pub listen_uri: String,
}
