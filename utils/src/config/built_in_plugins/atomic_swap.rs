use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct AtomicSwapConfig {
    enabled: bool,
}

impl AtomicSwapConfig {
    pub fn new(enabled: bool) -> Self {
        AtomicSwapConfig { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
