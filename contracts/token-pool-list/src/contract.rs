use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response};
use cw2::set_contract_version;
use cw_ownable::initialize_owner;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{Config, CONFIG, POOLS, TOKENS},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    initialize_owner(
        deps.storage,
        deps.api,
        Some(&info.sender.clone().into_string()),
    )?;

    let mut total_ratio = Decimal::zero();
    for dev_wallet in msg.dev_wallet_list.clone() {
        deps.api.addr_validate(&dev_wallet.address)?;
        total_ratio = total_ratio + dev_wallet.ratio;
    }

    if total_ratio != Decimal::one() {
        return Err(ContractError::WrongRatio {});
    }

    let config = Config {
        token_listing_fee: msg.token_listing_fee,
        pool_creation_fee: msg.pool_creation_fee,
        burn_fee_percent: msg.burn_fee_percent,
        dev_wallet_list: msg.dev_wallet_list,
    };

    CONFIG.save(deps.storage, &config)?;
    TOKENS.save(deps.storage, &msg.initial_tokens.unwrap_or(vec![]))?;
    POOLS.save(deps.storage, &msg.initial_pools.unwrap_or(vec![]))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("sender", info.sender))
}

pub mod query {
    use cosmwasm_std::{Deps, StdResult};

    use crate::{
        msg::{ConfigResp, PoolsResp, TokensResp},
        state::{CONFIG, POOLS, TOKENS},
    };

    pub fn token_list(deps: Deps) -> StdResult<TokensResp> {
        let tokens = TOKENS.load(deps.storage)?;
        Ok(TokensResp { tokens })
    }

    pub fn pool_list(deps: Deps) -> StdResult<PoolsResp> {
        let pools = POOLS.load(deps.storage)?;
        Ok(PoolsResp { pools })
    }

    pub fn config(deps: Deps) -> StdResult<ConfigResp> {
        let config = CONFIG.load(deps.storage)?;
        Ok(ConfigResp { config })
    }
}

pub mod exec {
    use std::str::FromStr;

    use cosmwasm_std::{
        to_binary, Coin, Decimal, DepsMut, MessageInfo, Response, StdError, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    use crate::{
        error::ContractError,
        state::{Chain, Config, Pool, Token, WalletInfo, CONFIG, POOLS, TOKENS},
    };

    pub fn create_pool(
        deps: DepsMut,
        info: MessageInfo,
        token1: String,
        token2: String,
        burn_ratio: u64,
        swap_fee: String,
    ) -> Result<Response, ContractError> {
        let mut pools = POOLS.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        let messages = charge_pool_creation_fee(config, info.clone())?;

        if burn_ratio >= 100 {
            return Err(ContractError::InvalidBurnRatio {});
        }
        if Decimal::from_str(&swap_fee).unwrap().floor() > Decimal::new(Uint128::new(10)) {
            return Err(ContractError::InvalidSwapFee {});
        };

        if pools.iter().any(|pool| {
            pool.creator == info.sender
                && pool.token1 == token1
                && pool.token2 == token2
                && pool.burn_ratio == burn_ratio
                && pool.swap_fee == swap_fee
        }) {
            return Err(ContractError::PoolExists {});
        } else {
            pools.push(Pool {
                token1: token1.clone(),
                token2: token2.clone(),
                creator: info.sender.clone(),
                burn_ratio,
                swap_fee: swap_fee.clone(),
            });
        }

        POOLS.save(deps.storage, &pools)?;

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "create_pool")
            .add_attribute("creator", info.sender)
            .add_attribute("token1", token1)
            .add_attribute("token2", token2)
            .add_attribute("burn_ratio", burn_ratio.to_string())
            .add_attribute("swap_fee", swap_fee))
    }

    pub fn list_token(
        deps: DepsMut,
        info: MessageInfo,
        denom: String,
        full_name: String,
        symbol: String,
        chain: Chain,
        is_native_coin: bool,
        is_ibc_coin: bool,
        decimal: u64,
        logo_uri: String,
    ) -> Result<Response, ContractError> {
        let mut tokens = TOKENS.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;

        let messages = charge_token_list_fee(config, info.clone())?;

        tokens.push(Token {
            denom: denom.clone(),
            full_name: full_name.clone(),
            symbol: symbol.clone(),
            chain,
            is_native_coin,
            is_ibc_coin,
            decimal,
            logo_uri,
        });
        
        TOKENS.save(deps.storage, &tokens)?;
        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "list_token")
            .add_attribute("denom", denom)
            .add_attribute("full_name", full_name)
            .add_attribute("symbol", symbol))
    }

    pub fn change_config(
        deps: DepsMut,
        info: MessageInfo,
        token_listing_fee: Coin,
        pool_creation_fee: Coin,
        burn_fee_percent: u64,
        dev_wallet_list: Vec<WalletInfo>,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let mut total_ratio = Decimal::zero();
        for dev_wallet in dev_wallet_list.clone() {
            deps.api.addr_validate(&dev_wallet.address)?;
            total_ratio = total_ratio + dev_wallet.ratio;
        }

        if total_ratio != Decimal::one() {
            return Err(ContractError::WrongRatio {});
        }

        let config = Config {
            token_listing_fee,
            pool_creation_fee,
            burn_fee_percent,
            dev_wallet_list,
        };

        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new().add_attribute("action", "changed_config"))
    }

    fn charge_pool_creation_fee(
        config: Config,
        info: MessageInfo,
    ) -> Result<Vec<WasmMsg>, ContractError> {
        let mut messages = vec![];

        let burn_fee_amount;
        if config.burn_fee_percent == 0 {
            burn_fee_amount = Uint128::new(0);
        } else {
            burn_fee_amount = config
                .pool_creation_fee
                .amount
                .checked_mul(Uint128::new(config.burn_fee_percent.into()))
                .map_err(StdError::overflow)?
                .checked_div(Uint128::new(100))
                .map_err(StdError::divide_by_zero)?;

            let burn_cw20_msg = Cw20ExecuteMsg::BurnFrom {
                owner: info.sender.clone().into_string(),
                amount: burn_fee_amount,
            };
            let exec_cw20_burn = WasmMsg::Execute {
                contract_addr: config.clone().pool_creation_fee.denom,
                msg: to_binary(&burn_cw20_msg)?,
                funds: vec![],
            };

            messages.push(exec_cw20_burn);
        }

        let transfer_amount = config.pool_creation_fee.amount - burn_fee_amount;

        for dev_wallet in config.clone().dev_wallet_list {
            let fee_amount = transfer_amount * dev_wallet.ratio;
            let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.clone().into_string(),
                recipient: dev_wallet.address,
                amount: fee_amount,
            };
            let exec_cw20_transfer = WasmMsg::Execute {
                contract_addr: config.clone().pool_creation_fee.denom,
                msg: to_binary(&transfer_cw20_msg)?,
                funds: vec![],
            };
            messages.push(exec_cw20_transfer);
        }

        Ok(messages)
    }

    fn charge_token_list_fee(
        config: Config,
        info: MessageInfo,
    ) -> Result<Vec<WasmMsg>, ContractError> {
        let mut messages = vec![];

        let burn_fee_amount;
        if config.burn_fee_percent == 0 {
            burn_fee_amount = Uint128::new(0);
        } else {
            burn_fee_amount = config
                .token_listing_fee
                .amount
                .checked_mul(Uint128::new(config.burn_fee_percent.into()))
                .map_err(StdError::overflow)?
                .checked_div(Uint128::new(100))
                .map_err(StdError::divide_by_zero)?;

            let burn_cw20_msg = Cw20ExecuteMsg::BurnFrom {
                owner: info.sender.clone().into_string(),
                amount: burn_fee_amount,
            };
            let exec_cw20_burn = WasmMsg::Execute {
                contract_addr: config.clone().pool_creation_fee.denom,
                msg: to_binary(&burn_cw20_msg)?,
                funds: vec![],
            };

            messages.push(exec_cw20_burn);
        }
        let transfer_amount = config.token_listing_fee.amount - burn_fee_amount;

        for dev_wallet in config.clone().dev_wallet_list {
            let fee_amount = transfer_amount * dev_wallet.ratio;
            let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.clone().into_string(),
                recipient: dev_wallet.address,
                amount: fee_amount,
            };
            let exec_cw20_transfer = WasmMsg::Execute {
                contract_addr: config.clone().pool_creation_fee.denom,
                msg: to_binary(&transfer_cw20_msg)?,
                funds: vec![],
            };
            messages.push(exec_cw20_transfer);
        }

        Ok(messages)
    }
}
