use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct CkbConfig {
    network_type: String,
    ckb_uri: String,
}

impl CkbConfig {
    pub fn new(network_type: &str, ckb_uri: &str) -> Self {
        CkbConfig {
            network_type: network_type.to_string(),
            ckb_uri: ckb_uri.to_string(),
        }
    }

    pub fn get_ckb_uri(&self) -> &str {
        &self.ckb_uri
    }

    pub fn get_network_type(&self) -> &str {
        &self.network_type
    }
}
