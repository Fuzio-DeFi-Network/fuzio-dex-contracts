use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal};
use cw_storage_plus::Item;

#[cw_serde]
pub struct GasPrice {
    pub denom: String,
    pub amount: u64,
}
#[cw_serde]
pub struct IbcChannels {
    pub deposit_channel: String,
    pub withdraw_channel: String,
}

#[cw_serde]
pub struct Chain {
    pub chain_name: String,
    pub chain_id: String,
    pub gas_price: GasPrice,
    pub ibc_channels: Option<IbcChannels>,
    pub is_evm: bool,
}

#[cw_serde]
pub struct Token {
    pub denom: String,
    pub full_name: String,
    pub symbol: String,
    pub chain: Chain,
    pub is_native_coin: bool,
    pub is_ibc_coin: bool,
    pub decimal: u64,
    pub logo_uri: String,
}

#[cw_serde]
pub struct Pool {
    pub token1: String,
    pub token2: String,
    pub creator: Addr,
    pub burn_ratio: u64,
    pub swap_fee: String,
}

#[cw_serde]
pub struct WalletInfo {
    pub address: String,
    pub ratio: Decimal,
}

#[cw_serde]
pub struct Config {
    pub token_listing_fee: Coin,
    pub pool_creation_fee: Coin,
    pub burn_fee_percent: u64,
    pub dev_wallet_list: Vec<WalletInfo>, 
}

pub const TOKENS: Item<Vec<Token>> = Item::new("token_list");
pub const POOLS: Item<Vec<Pool>> = Item::new("pools_list");
pub const CONFIG: Item<Config> = Item::new("fees");
