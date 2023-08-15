# Fuzio-Pool

This contract is a slightly modified version of [WasmSwap](https://github.com/Wasmswap/wasmswap-contracts) but with an array of dev wallets that will receive a ratio of the dev rewards and with updated Cosmwasm dependencies.

# Instantiation

The contract can be instantiated with the following messages

```
{
    "token1_denom": {"native": "<DENOM>"},
    "token2_denom": {"cw20": "<CONTRACT_ADDRESS>"},
    "lp_token_code_id": '<CW20_CODE_ID>',
    "owner": "<OWNER_ADDRES>",
    "fee_percent_numerator": "<FEE_PERCENT_NUMERATOR>",
    "fee_percent_denominator": "<FEE_PERCENT_DENOMINATOR>",
    "lp_token_name": "<LP_TOKEN_NAME>",
    "lp_token_symbol": "<LP_TOKEN_SYMBOL>"
    "dev_wallet_lists": [{"address": "<DEV_WALLET_1>", "ratio": "<RATIO_1>"},...]
}
```

Token denom can be either `native` for tokens tracked by the bank module (including IBC assets or created with token factory) or `cw20` for cw20 tokens. `native` tokens have a denom string and `cw20` tokens have a contract address. `CW20_CODE_ID` is the code id for a basic cw20 binary.

# Messages

### Add Liquidity

Allows a user to add liquidity to the pool.

### Remove Liquidity

Allows a user to remove liquidity from the pool.

### Swap

Swap one asset for the other

### Pass Through Swap

Execute a multi contract swap where A is swapped for B and then B is sent to another contract where it is swapped for C.

### Swap And Send To

Execute a swap and send the new asset to the given recipient. This is mostly used for `PassThroughSwaps`.
