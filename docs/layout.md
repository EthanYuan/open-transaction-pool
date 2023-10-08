# otx-pool layout

```sh
otx-pool
├── otx-format 
├── otx-plugin-protocol 
├── otx-pool-service
├── otx-sdk
├── plugins-built-in
├── src
│   └── main.rs
├── util
├── docs
└── integration-test
```

A brief description:

- `otx-format` contains the implementation of the data structure of [CKB Open Transaction Format](https://github.com/nervosnetwork/rfcs/pull/406) and the surrounding tools.
- `otx-plugin-protocol ` defines the protocol for extensible plugins.
- `otx-pool-service` open transaction memory pool implementation, it supports plugin extension.
- `otx-sdk` includes tools for constructing transactions, signing, and more, based on the OTX lock script.
- `plugins-built-in` various built-in plugins are placed as independent crates in this directory.
- `src` contains main.rs, the entry point of the service program.
- `util` contains various utilities that are used both on the server side and on the wallet side.
- `docs` contains project documentations.
- `integration-test` is the integration tests for this project.
