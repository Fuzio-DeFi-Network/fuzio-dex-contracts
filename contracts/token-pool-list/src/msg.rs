use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Coin;

use crate::state::Chain;
use crate::state::Config;
use crate::state::Pool;
use crate::state::Token;
use crate::state::WalletInfo;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(TokensResp)]
    Tokens {},
    #[returns(PoolsResp)]
    Pools {},
    #[returns(ConfigResp)]
    Config {},
}

#[cw_serde]
pub struct TokensResp {
    pub tokens: Vec<Token>,
}

#[cw_serde]
pub struct PoolsResp {
    pub pools: Vec<Pool>,
}

#[cw_serde]
pub struct ConfigResp {
    pub config: Config,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub token_listing_fee: Coin,
    pub pool_creation_fee: Coin,
    pub burn_fee_percent: u64,
    pub dev_wallet_list: Vec<WalletInfo>,
    pub initial_tokens: Option<Vec<Token>>,
    pub initial_pools: Option<Vec<Pool>>,
}

#[cw_serde]
pub enum ExecMsg {
    CreatePool {
        token1: String,
        token2: String,
        burn_ratio: u64,
        swap_fee: String,
    },
    ListToken {
        denom: String,
        full_name: String,
        symbol: String,
        chain: Chain,
        is_native_coin: bool,
        is_ibc_coin: bool,
        decimal: u64,
        logo_uri: String,
    },
    ChangeConfig {
        token_listing_fee: Coin,
        pool_creation_fee: Coin,
        burn_fee_percent: u64,
        dev_wallet_lists: Vec<WalletInfo>,
    },
}
