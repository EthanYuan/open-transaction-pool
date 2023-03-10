# OTX Pool and Plugin

##  1 Basic Architecture

We believe that the optimal architecture involves processing Open Transaction (OTX) as a data stream. This architecture utilizes a Broker to collect OTXs and dispatch them to Agents.

An Agent serves as both a Consumer and a Producer. It receives OTXs from the Broker, executes its logic, and notifies the Broker of its processing result.

This project is an implementation of this design concept. It uses the memory pool as the Broker and plugins as Agents to expand its application business logic.

The design of the plugins in this project utilizes a communication mechanism between the host and the plugin that works in both directions. For built-in plugins, inter-thread communication is used, while for external plugins, inter-process communication based on stdin and stdout is used.

## 2 OTX Pool

For plugins, the OTX Pool acts as the host and provides the following specific functionalities:

-   Provides RPC services, such as submitting and querying OTXs
-   Manages the subscription of events
-   Generates and notifies events
-   Provides host services that can be invoked by plugins

This design gives the pool the characteristics of a development framework.

## 3 Plugin Protocol

The term "message" is used to describe the information that is exchanged between a host and a plugin. This can include notifications, requests or responses. Messages can be distinguished by two different directions: those that flow from the host to the plugin, and those that flow from the plugin to the host.

```rust
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
```

```rust
pub enum MessageFromPlugin {
    // Response
    Ok,
    Error(String),
    PluginInfo(PluginInfo),

    // Request
    NewOtx(OpenTransaction),
    DiscardOtx((H256, OpenTransaction)),
    ModifyOtx((H256, OpenTransaction)),
    SendCkbTx((H256, Vec<H256>)),
}
```

## 4 External Plugin

OTX pool communicates with plugins by starting a plugin process and using stdin/stdout for reading/writing requests and responses. Therefore, plugins can be written in any programming language, and a crashing plugin should not cause the OTX pool process to crash.

External plugins are standalone programs that must be installed and activated before they can work properly, unlike built-in plugins. Once activated, the host will launch the plugin as a daemon process, allowing it to run continuously in the background.

At startup, the host scans a specified directory to obtain basic information about all installed plugins, including inactive ones. The basic mechanism for obtaining plugin information is to temporarily start the plugin process and initiate a `GetPluginInfo` request.

Plugins that comply with the communication protocol will return the following data structure:

```rust
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}
```

## 5 Built-in Plugin

Like external plugins, they share a common communication protocol, making no difference to plugin managers. However, unlike external plugins, internal plugins do not require installation and are implemented as a sub-thread of the host program.

```rust
pub trait Plugin {
    fn get_name(&self) -> String;
    fn request_handler(&self) -> RequestHandler;
    fn msg_handler(&self) -> MsgHandler;
    fn get_info(&self) -> PluginInfo;
    fn get_state(&self) -> PluginState;
}
```

The two plugins that have been implemented so far, [Dust Collector](../otx-pool/src/built_in_plugin/dust_collector.rs) and [Atomic Swap](../otx-pool/src/built_in_plugin/atomic_swap.rs), are both internal plugins.

## 6 Host Service

Plugins also require access to data and functions provided by the host. Therefore, a host service is implemented to listen for plugin requests and handle them in a separate thread.

