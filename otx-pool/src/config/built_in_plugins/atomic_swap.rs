use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug)]
pub struct AtomicSwapConfig {
    pub enabled: bool,
}
