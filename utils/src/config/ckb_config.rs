use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct CkbConfig {
    pub network_type: String,
    pub ckb_uri: String,
}
