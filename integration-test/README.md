## Run integration tests for the first time


### Install CKB

- [install ckb](https://docs.nervos.org/docs/basics/guides/get-ckb/#build-from-source) (compile and install with the latest release version)
- [install ckb-cli](https://github.com/nervosnetwork/ckb-cli#build-this-project)

### Init Mercury


- install and start PostgreSQL
- create new database `mercury-otx-dev`, if it already exists, delete it first and then re-create it
- create tables and indexes

```bash
cd integration-test
psql -h localhost -U postgres -d mercury-otx-dev -f devtools/create_table.sql
```

### Init CKB

```bash
cd integration-test
rm -rf ./dev_chain/dev/data  ./free-space
```

Please ensure that the config of ckb-cli is set to the local network `http://127.0.0.1:8114`.

```bash
ckb-cli
```

```bash
CKB> config --url http://127.0.0.1:8114
```

### Run integration tests

```bash
cd integration-test
cargo run
```

or
 
```bash
cd integration-test
cargo run -- -t test_service_rpc
```

If there is no new contract to be deployed to the genesis block, the previous preparation work is no longer required, and the integration test can be run directly.

## Deploy a new contract to the genesis block

Currently integration tests are based on dev chain
, the contracts deployed on it are all implemented through the genesis block config declaration.

If you need to deploy contract scripts on the dev chain, you need to do the following:

- init database (Same as previous section)
- put the compiled contract binary into the specified location

    ```bash
    dev_chain/dev/specs/cells
    ```

- update `dev.toml`: add new script information

    ```toml
    [[genesis.system_cells]]
    file = { file = "cells/omni_lock" }
    create_type_id = true
    capacity = 200_555_0000_0000
    ```

- init CKB (Same as previous section)
- run CKB node and get transactions in genesis block

    After completing the initialization of ckb, you can start the ckb node independently.


    ```bash
    ckb run -C dev_chain/dev --skip-spec-check
    ```

    Then you can directly call CKB's RPC `get_block_by_number`.

    ```bash
    echo '{
    "id": 42,
    "jsonrpc": "2.0",
    "method": "get_block_by_number",
    "params": [
        "0x0"
    ]
    }' \
    | tr -d '\n' \
    | curl -H 'content-type: application/json' -d @- http://127.0.0.1:8114 > genesis.json
    ```

- update the existing configuration according to the genesis transactions in both `devnet_config.toml` and `mercury_devnet_config.toml`

- add new script in `devnet_config.toml`, for example:

    ```toml
    [[scripts]]
    script_name = "otx-sighash-lock"
    script = '''
    {
        "args": "0x",
        "code_hash": "0xddefa1e2cede14bd25f92143f7f4ca3af6fa5ac1969c53cb3c3914c9f1cded96",
        "hash_type": "type"
    }
    '''
    cell_dep = '''
    {
        "dep_type": "dep_group",
        "out_point": {
            "index": "0x4",
            "tx_hash": "0x8d47e8719ae7a7c27785babd837d17454a48e6f353ddfe4bdfe30ccf33aacca5"
        }
    }
    ```

    The following code is the algorithm to calculate the code hash in the above config:

    ```rust
    #[cfg(test)]
    mod tests {
        use ckb_types::core::ScriptHashType;
        use ckb_types::packed;
        use ckb_types::prelude::*;
        use ckb_types::H256;

        use std::str::FromStr;

        fn caculate_type_hash(code_hash: &str, args: &str, script_hash_type: ScriptHashType) -> H256 {
            let code_hash = H256::from_str(code_hash).unwrap();
            let args = H256::from_str(args).unwrap();
            let script = packed::Script::new_builder()
                .hash_type(script_hash_type.into())
                .code_hash(code_hash.pack())
                .args(ckb_types::bytes::Bytes::from(args.as_bytes().to_owned()).pack())
                .build();
            script.calc_script_hash().unpack()
        }

        #[test]
        fn test_caculate_script_hash() {
            let code_hash = "00000000000000000000000000000000000000000000000000545950455f4944";
            let args = "02df593065ff5d52c90aabf799433cfcbf0147fc2f7b649688026d4d4ec62d5e";
            let script_hash_type = ScriptHashType::Type;

            let script_hash = caculate_type_hash(code_hash, args, script_hash_type);
            println!("{:?}", script_hash.to_string());
        }
    }
    ```

- run integration tests

    ```bash
    cd integration-test
    cargo run
    ```

