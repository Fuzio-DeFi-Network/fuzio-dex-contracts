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
    pub burn_fee_percent: u64,
    pub dev_wallet_list: Vec<WalletInfo>, 
}
```

Each time a pool is created or a token is listed, a certain fee is charged to the creator/lister and part of it is burnt and the other part is sent to a list of wallets.