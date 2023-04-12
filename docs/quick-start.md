
# Quick Start

This guide will show you how to set up Open Transaction Pool. All the steps in this guide are using Ubuntu 20.04 LTS.

You can build Open Transaction Pool from the source code by following these steps:

## Building From Source

```sh
git clone https://github.com/EthanYuan/open-transaction-pool.git
cd open-transaction-pool
cargo build --release
```

After cloning the repository and running the build command, you can proceed to configure the service.

## Configure

### Prepared configuration files

If your local development environment is deployed according to the instructions in the [integration-tests README](./integration-test/README.md), you can use this configuration file directly:

```sh
# devnet
nano ./integration-test/dev_chain/devnet_config.toml 
```

The configuration files for the mainnet and testnet are being prepared, and the corresponding scripts need to be deployed before they can be used.

```sh
# mainnet
# in preparation, will be available soon
nano ./src/configs/mainnet_config.toml 
```

```sh
# testnet
# in preparation, will be available soon
nano ./src/configs/testnet_config.toml 
```

### Update the CKB Node RPC URI

```toml
[ckb_config]
network_type = "ckb_dev"
ckb_uri = "http://127.0.0.1:8114"
```

### Update the Listen URI

```toml
[network_config]
listen_uri = "http://127.0.0.1:8118"
```

### Update the configuration of built-in plugins

```toml
[built_in_plugin_dust_collector]
enabled = true
default_address = "DEFAUT_ADDRESS"

[built_in_plugin_signer]
enabled = true
key = "PRIVKEY"
default_address = "DEFAUT_ADDRESS"
```

After updating the configuration files, you can proceed to start the Open Transaction Pool service.

## Running Service

First, if the enabled built-in plugin requires environment variables, the name in the configuration file needs to be used to set the environment variables.

```bash
export PRIVKEY="ef4dfe655b3df20838bdd16e20afc70dfc1b9c3e87c54c276820315a570e6555"
export DEFAUT_ADDRESS="ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqf7v2xsyj0p8szesqrwqapvvygpc8hzg9sku954v"
```

Then start the OTX pool service:

```sh
target/release/open-transaction-pool --config-path integration-test/dev_chain/devnet_config.toml
```

**NOTE**: The address and key are for demo purposes only and should not be used in a production environment.
