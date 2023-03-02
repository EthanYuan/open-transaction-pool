# open-transaction-pool layout

```sh
open-transaction-pool
├── otx-format 
├── otx-plugin-protocol 
├── otx-pool 
├── utils 
├── src
|   └── main.rs
├── docs
|   └── layout.md
└── integration-test
```

A brief description:

- `otx-format` contains the implementation of the data structure of [CKB Open Transaction Format](https://github.com/nervosnetwork/rfcs/pull/406) and the surrounding tools.
- `otx-plugin-protocol ` defines the protocol for extensible plug-ins.
- `otx-pool ` open transaction pool implementation, it supports plug-in extension.
- `utils` contains various utilities that are used both on the server side and on the wallet side.
- `src` contains main.rs, the entry point of the service program.
- `docs` contains project documentations.
- `integration-test` is the integration tests for this project.
