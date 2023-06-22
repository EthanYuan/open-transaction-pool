/// NOTE: this example is for plugin integration tests
use otx_pool_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};

use std::io::{self, Write};

fn main() {
    loop {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                break;
            }
            Ok(_) => {
                let (id, msg): (u64, MessageFromHost) = serde_json::from_str(&line).unwrap();
                if let Some(msg) = handle(msg) {
                    let response_string =
                        format!("{}\n", serde_json::to_string(&(id, msg)).unwrap());
                    io::stdout().write_all(response_string.as_bytes()).unwrap();
                    io::stdout().flush().unwrap();
                }
            }
            Err(_err) => {}
        }
    }
}

fn handle(msg: MessageFromHost) -> Option<MessageFromPlugin> {
    match msg {
        MessageFromHost::GetPluginInfo => {
            let info = PluginInfo {
                name: String::from("plugin demo"),
                description: String::from("It's a plugin demo"),
                version: 0.to_string(),
            };
            Some(MessageFromPlugin::PluginInfo(info))
        }
        MessageFromHost::NewInterval(_) => {
            log::info!("New interval");
            Some(MessageFromPlugin::Ok)
        }
        _ => None,
    }
}
