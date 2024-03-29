use otx_format::jsonrpc_types::OpenTransaction;

use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::Sender;
use serde_derive::{Deserialize, Serialize};

use std::path::PathBuf;

pub trait Plugin: Send + Sync {
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

#[derive(Clone, Debug)]
pub struct PluginMeta {
    /// The installation path of the plug-in, the built-in plugin binary_path is default value.
    pub binary_path: PathBuf,
    /// Activation falg.
    pub is_active: bool,
    /// Built-in flag.
    pub is_built_in: bool,
}

impl PluginMeta {
    pub fn new(binary_path: PathBuf, is_active: bool, is_built_in: bool) -> PluginMeta {
        PluginMeta {
            binary_path,
            is_active,
            is_built_in,
        }
    }
}

pub type HostServiceHandler = Sender<Request<MessageFromPlugin, MessageFromHost>>;

pub enum MessageType {
    Request,
    Response,
    Notify,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageFromHost {
    // Notify
    NewOtx(OpenTransaction),
    NewInterval(u64),
    OtxPoolStart,
    OtxPoolStop,
    CommitOtx(Vec<H256>),

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
            | Self::NewInterval(_)
            | Self::OtxPoolStart
            | Self::OtxPoolStop
            | Self::CommitOtx(_) => MessageType::Notify,
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
    NewMergedOtx((OpenTransaction, Vec<H256>)),
    DiscardOtx((H256, OpenTransaction)),
    ModifyOtx((H256, OpenTransaction)),
    SentToCkb(H256),
    MergeOtxsAndSentToCkb((Vec<H256>, H256)),
}

impl MessageFromPlugin {
    pub fn get_message_type(&self) -> MessageType {
        match self {
            Self::Ok | Self::Error(_) | Self::PluginInfo(_) => MessageType::Response,
            Self::NewMergedOtx(_)
            | Self::DiscardOtx(_)
            | Self::ModifyOtx(_)
            | Self::SentToCkb(_)
            | Self::MergeOtxsAndSentToCkb(_) => MessageType::Request,
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
