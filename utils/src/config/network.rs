use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct NetworkConfig {
    listen_uri: String,
}

impl NetworkConfig {
    pub fn new(listen_uri: &str) -> Self {
        NetworkConfig {
            listen_uri: listen_uri.to_string(),
        }
    }

    pub fn get_listen_uri(&self) -> &str {
        &self.listen_uri
    }
}
