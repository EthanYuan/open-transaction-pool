pub mod host_service;
pub mod manager;
pub mod plugin_proxy;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_pool_plugin_protocol::{PluginInfo, PluginMeta};

use ckb_types::H256;

pub trait Plugin: Send {
    fn get_name(&self) -> String;
    fn get_meta(&self) -> PluginMeta;
    fn get_info(&self) -> PluginInfo;
    fn on_new_otx(&self, _otx: OpenTransaction) {
        // This is a default implementation that does nothing.
    }
    fn on_new_intervel(&self, _interval: u64) {
        // This is a default implementation that does nothing.
    }
    fn on_commit_otx(&self, _otxs: Vec<H256>) {
        // This is a default implementation that does nothing.
    }
}
