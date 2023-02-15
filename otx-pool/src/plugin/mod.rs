pub mod host_service;
pub mod manager;
pub mod plugin_proxy;

use otx_plugin_protocol::PluginInfo;
use plugin_proxy::{MsgHandler, PluginState, RequestHandler};

pub trait Plugin {
    fn get_name(&self) -> String;
    fn request_handler(&self) -> RequestHandler;
    fn msg_handler(&self) -> MsgHandler;
    fn get_info(&self) -> PluginInfo;
    fn get_state(&self) -> PluginState;
}
