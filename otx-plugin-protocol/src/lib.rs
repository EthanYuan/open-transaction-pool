use otx_format::jsonrpc_types::OpenTransaction;

use serde_derive::{Deserialize, Serialize};

pub type Id = u64;

pub enum MessageType {
    Request,
    Response,
    Notify,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageFromHost {
    // Notify
    NewOtx(OpenTransaction),
    NewInterval,
    OtxPoolStart,
    OtxPoolStop,
    DeleteOtx(Id),

    // Request
    GetPluginInfo,

    // Response
    Ok,
    Error(String),
}

impl MessageFromHost {
    pub fn get_message_type(&self) -> MessageType {
        match self {
            Self::NewOtx(_)
            | Self::NewInterval
            | Self::OtxPoolStart
            | Self::OtxPoolStop
            | Self::DeleteOtx(_) => MessageType::Notify,
            Self::GetPluginInfo | Self::Ok | Self::Error(_) => MessageType::Request,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageFromPlugin {
    // Response
    Ok,
    Error(String),
    PluginInfo(PluginInfo),

    // Request
    NewOtx(OpenTransaction),
    DiscardOtx(Id),
    ModifyOtx((Id, OpenTransaction)),
    SendCkbTx(OpenTransaction),
}

impl MessageFromPlugin {
    pub fn get_message_type(&self) -> MessageType {
        match self {
            Self::Ok | Self::Error(_) | Self::PluginInfo(_) => MessageType::Response,
            Self::NewOtx(_) | Self::DiscardOtx(_) | Self::ModifyOtx(_) | Self::SendCkbTx(_) => {
                MessageType::Request
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}

impl PluginInfo {
    pub fn new(name: &str, description: &str, version: &str) -> Self {
        PluginInfo {
            name: name.into(),
            description: description.into(),
            version: version.into(),
        }
    }
}
