# Fuzio DEX Contracts

This package contains the contracts for the IDO functionality of Fuzio.

## Contracts

| Name                                   | Description             |
| -------------------------------------- | ----------------------- |
| [`fuzio_staking`](contracts/fuzio-staking)   | Fuzio LP token staking |
| [`fuzio-pool`](contracts/fuzio-pool)   | Pool contract |
| [`token-pool-list`](contracts/token-pool-list)   | Pools and tokens information |

To compile all contracts in the workspace deterministically, you can run:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.14.0
```

To generate all the schema files run:

```bash
sh scripts/schema.sh 
```
