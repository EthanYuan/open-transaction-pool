use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct NetworkConfig {
    pub listen_uri: String,
}
