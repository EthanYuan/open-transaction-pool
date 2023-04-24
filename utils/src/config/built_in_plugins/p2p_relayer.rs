use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct P2PRelayerConfig {
    enabled: bool,
}

impl P2PRelayerConfig {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
