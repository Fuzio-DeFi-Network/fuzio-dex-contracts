# Token-Pool-List

This is a contract to store all the pool and token information of Fuzio DEX to allow permissionless listing of tokens and permissionless creation of pools.

THe contract will be instantiated with a set up Config that can be modified by the owner:

```rust
pub struct WalletInfo {
    pub address: String,
    pub ratio: Decimal,
}

pub struct Config {
    pub token_listing_fee: Coin,
    pub pool_creation_fee: Coin,
    pub burn_fee_percent: u64, // 1 = 1%
    pub dev_wallet_list: Vec<WalletInfo>, 
}
```

Each time a pool is created or a token is listed, a certain fee is charged to the creator/lister and part of it is burnt and the other part is sent to a list of wallets.

# Instantiation

The contract can be instantiated with the following messages

```
{
    "token_listing_fee": {"denom": "<DENOM_FEE>", "amount": "<AMOUNT_FEE>"},
    "pool_creation_fee": {"denom": "<DENOM_FEE>", "amount": "<AMOUNT_FEE>"},
    "burn_fee_percent": '<BURN_PERCENT>',
    "dev_wallet_list": [{"address": "<DEV_WALLET_1>", "ratio": "<RATIO_1>"},...]
    "initial_tokens": [<TOKEN1>, ...] //OPTIONAL
    "initial_pools": [<POOL1>, ...] //OPTIONAL
}
```

# Messages

### CreatePool

Adds a Pool to the list

### ListToken

Adds a Token to the list

### ChangeConfig (Admin only)

Allows the admin to modify the config of the list