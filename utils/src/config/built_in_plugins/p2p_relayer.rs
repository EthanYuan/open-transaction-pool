use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct P2PRelayerConfig {
    enabled: bool,
    listen: Option<String>,
    dial: Option<String>,
}

impl P2PRelayerConfig {
    pub fn new(enabled: bool, listen: Option<String>, dial: Option<String>) -> Self {
        Self {
            enabled,
            listen,
            dial,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn listen(&self) -> Option<&str> {
        self.listen.as_deref()
    }

    pub fn dial(&self) -> Option<&str> {
        self.dial.as_deref()
    }
}
