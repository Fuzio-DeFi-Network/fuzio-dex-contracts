mod contract;
pub mod error;
pub mod msg;
mod state;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use error::ContractError;
use msg::{ExecMsg, InstantiateMsg, QueryMsg};

use crate::contract::{
    exec::{create_pool, list_token, change_config},
    query::{config, pool_list, token_list},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    contract::instantiate(deps, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use crate::msg::QueryMsg::*;
    match msg {
        Tokens {} => to_binary(&token_list(deps)?),
        Pools {} => to_binary(&pool_list(deps)?),
        Config {} => to_binary(&config(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecMsg,
) -> Result<Response, ContractError> {
    use msg::ExecMsg::*;

    match msg {
        CreatePool {
            token1,
            token2,
            burn_ratio,
            swap_fee,
        } => create_pool(deps, info, token1, token2, burn_ratio, swap_fee),
        ListToken {
            denom,
            full_name,
            symbol,
            chain,
            is_native_coin,
            is_ibc_coin,
            decimal,
            logo_uri,
        } => list_token(
            deps,
            info,
            denom,
            full_name,
            symbol,
            chain,
            is_native_coin,
            is_ibc_coin,
            decimal,
            logo_uri,
        ),
        ChangeConfig {
            token_listing_fee,
            pool_creation_fee,
            burn_fee_percent,
            dev_wallet_lists,
        } => change_config(deps, info, token_listing_fee, pool_creation_fee, burn_fee_percent, dev_wallet_lists),
    }
}
