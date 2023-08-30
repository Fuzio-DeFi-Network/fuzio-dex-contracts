use cosmwasm_std::{
    entry_point, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::query::query_all_unbonding_info;
use crate::state::{
    staker_info_key, staker_info_storage, unbonding_info_key, unbonding_info_storage,
    user_earned_info_key, user_earned_info_storage, Config, Denom, Schedule, StakerInfo, State,
    UnbondingInfo, UserEarnedInfo, CONFIG, STATE,
};

use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use std::collections::BTreeMap;
use std::vec;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    deps.api.addr_validate(&msg.lp_token_contract)?;

    if msg.reward_token.len() != msg.distribution_schedule.len() {
        return Err(ContractError::InvalidSchedules {});
    }

    CONFIG.save(
        deps.storage,
        &Config {
            lp_token_contract: msg.clone().lp_token_contract,
            reward_token: msg.clone().reward_token,
            distribution_schedule: msg.distribution_schedule,
            admin: info.sender.to_string(),
            lock_duration: msg.lock_duration,
        },
    )?;

    STATE.save(
        deps.storage,
        &State {
            last_distributed: env.block.time.seconds(),
            total_bond_amount: Uint128::zero(),
            global_reward_index: vec![Decimal::zero(); msg.reward_token.len()],
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unbond { amount } => unbond(deps, env, info, amount),
        ExecuteMsg::Redeem {} => redeem(deps, env, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
        ExecuteMsg::UpdateConfig {
            distribution_schedule,
        } => update_config(deps, env, info, distribution_schedule),
        ExecuteMsg::UpdateAdmin { admin } => update_admin(deps, info, admin),
        ExecuteMsg::UpdateTokenContract {
            lp_token_contract,
            reward_token,
        } => update_token_contract(deps, info, lp_token_contract, reward_token),
        ExecuteMsg::UpdateLockDuration { lock_duration } => {
            update_lock_duration(deps, info, lock_duration)
        }
        ExecuteMsg::UpdateTokensAndDistribution {
            lp_token_contract,
            reward_token,
            distribution_schedule,
        } => update_tokens_and_distribution(
            deps,
            info,
            lp_token_contract,
            reward_token,
            distribution_schedule,
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_contract = info.sender.to_string();

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Bond {}) => {
            // only staking token contract can execute this message
            if config.lp_token_contract != token_contract {
                return Err(ContractError::WrongContractError {});
            }

            let cw20_sender = cw20_msg.sender;
            bond(deps, env, cw20_sender, cw20_msg.amount)
        }
        Err(_) => return Err(ContractError::DataShouldBeGiven {}),
    }
}

pub fn bond(
    deps: DepsMut,
    env: Env,
    sender_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let staker_info_key = staker_info_key(&sender_addr);
    let mut staker_info: StakerInfo;

    let len = config.reward_token.len();

    match staker_info_storage().may_load(deps.storage, staker_info_key.clone())? {
        Some(some_staker_info) => staker_info = some_staker_info,
        None => {
            staker_info = StakerInfo {
                reward_index: vec![Decimal::zero(); len],
                bond_amount: Uint128::zero(),
                pending_reward: vec![Uint128::zero(); len],
                address: sender_addr.clone(),
            }
        }
    };

    // Compute global reward & staker reward
    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;

    // Increase bond_amount
    increase_bond_amount(&mut state, &mut staker_info, amount);

    // Store updated state with staker's staker_info
    staker_info_storage().save(deps.storage, staker_info_key.clone(), &staker_info)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "bond"),
        ("owner", sender_addr.as_str()),
        ("amount", amount.to_string().as_str()),
    ]))
}

pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let sender_addr = info.sender.to_string();
    let time = env.block.time.seconds();

    let staker_info_key = staker_info_key(&sender_addr);
    let mut staker_info: StakerInfo;
    match staker_info_storage().may_load(deps.storage, staker_info_key.clone())? {
        Some(some_staker_info) => staker_info = some_staker_info,
        None => return Err(ContractError::NotStaked {}),
    };

    if staker_info.bond_amount < amount {
        return Err(ContractError::ExceedBondAmount {});
    }

    // Compute global reward & staker reward
    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;

    // Decrease bond_amount
    decrease_bond_amount(&mut state, &mut staker_info, amount)?;

    // Store or remove updated rewards info
    // depends on the left pending reward and bond amount
    if staker_info
        .pending_reward
        .iter()
        .all(|amount| amount.is_zero())
        && staker_info.bond_amount.is_zero()
    {
        staker_info_storage().remove(deps.storage, staker_info_key)?;
    } else {
        staker_info_storage().save(deps.storage, staker_info_key, &staker_info)?;
    }

    // Store updated state
    STATE.save(deps.storage, &state)?;

    let unbonding_info_key = unbonding_info_key(&sender_addr, time);
    unbonding_info_storage().save(
        deps.storage,
        unbonding_info_key,
        &UnbondingInfo {
            address: sender_addr.clone(),
            amount,
            time,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "unbond"),
        ("owner", info.sender.as_str()),
        ("amount", amount.to_string().as_str()),
    ]))
}

pub fn redeem(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender_addr = info.sender.to_string();
    let crr_time = env.block.time.seconds();

    let config = CONFIG.load(deps.storage)?;

    let mut amount = Uint128::zero();

    let unbonding_infos = query_all_unbonding_info(deps.as_ref(), env, sender_addr.clone())?;
    for unbonding_info in unbonding_infos {
        if unbonding_info.time + config.lock_duration > crr_time {
            break;
        } else {
            amount += unbonding_info.amount;
            let unbonding_info_key = unbonding_info_key(&sender_addr, unbonding_info.time);
            unbonding_info_storage().remove(deps.storage, unbonding_info_key.clone())?;
        }
    }

    if amount.is_zero() {
        return Err(ContractError::NothingToRedeem {});
    }

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.lp_token_contract,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            ("action", "redeem"),
            ("owner", info.sender.as_str()),
            ("amount", amount.to_string().as_str()),
        ]))
}

// withdraw rewards to executor
pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender_addr = info.sender.to_string();

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let staker_info_key = staker_info_key(&sender_addr);
    let mut staker_info: StakerInfo;
    match staker_info_storage().may_load(deps.storage, staker_info_key.clone())? {
        Some(some_staker_info) => staker_info = some_staker_info,
        None => return Err(ContractError::NotStaked {}),
    };

    // Compute global reward & staker reward
    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;

    let user_earned_info_key = user_earned_info_key(&sender_addr);
    match user_earned_info_storage().may_load(deps.storage, user_earned_info_key.clone())? {
        Some(mut user_earned_info) => {
            for (index, amount) in staker_info.pending_reward.iter().enumerate() {
                user_earned_info.total_earned[index] += amount;
            }
            user_earned_info_storage().save(
                deps.storage,
                user_earned_info_key,
                &user_earned_info,
            )?;
        }
        None => {
            user_earned_info_storage().save(
                deps.storage,
                user_earned_info_key,
                &UserEarnedInfo {
                    address: sender_addr,
                    total_earned: staker_info.clone().pending_reward,
                },
            )?;
        }
    }

    let len = staker_info.pending_reward.len();
    let amounts = staker_info.pending_reward;
    staker_info.pending_reward = vec![Uint128::zero(); len];

    // Store or remove updated rewards info
    // depends on the left pending reward and bond amount
    if staker_info.bond_amount.is_zero() {
        staker_info_storage().remove(deps.storage, staker_info_key)?;
    } else {
        staker_info_storage().save(deps.storage, staker_info_key, &staker_info)?;
    }

    // Store updated state
    STATE.save(deps.storage, &state)?;

    let mut reward_msgs = vec![];

    for (index, denom) in config.reward_token.iter().enumerate() {
        let reward_msg;
        match denom {
            Denom::Native(denom) => {
                reward_msg = CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: vec![Coin {
                        denom: denom.to_string(),
                        amount: amounts[index],
                    }],
                })
            }
            Denom::Cw20(address) => {
                reward_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: address.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.to_string(),
                        amount: amounts[index],
                    })?,
                    funds: vec![],
                });
            }
        }
        reward_msgs.push(reward_msg);
    }
    Ok(Response::new()
        .add_messages(reward_msgs)
        .add_attributes(vec![
            ("action", "withdraw"),
            ("owner", info.sender.as_str()),
        ]))
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    distribution_schedule: Vec<Schedule>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    authcheck(deps.as_ref(), &info)?;

    assert_new_schedules(&config, &state, distribution_schedule.clone())?;

    let new_config = Config {
        admin: config.admin,
        lp_token_contract: config.lp_token_contract,
        reward_token: config.reward_token,
        distribution_schedule,
        lock_duration: config.lock_duration,
    };
    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::new().add_attributes(vec![("action", "update_config")]))
}

pub fn update_admin(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;

    authcheck(deps.as_ref(), &info)?;
    config.admin = address;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![("action", "update_admin")]))
}

pub fn update_token_contract(
    deps: DepsMut,
    info: MessageInfo,
    lp_contract: String,
    reward_token: Vec<Denom>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&lp_contract)?;

    authcheck(deps.as_ref(), &info)?;

    if reward_token.len() != config.distribution_schedule.len() {
        return Err(ContractError::InvalidSchedules {});
    }

    config.reward_token = reward_token;
    config.lp_token_contract = lp_contract;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![("action", "update_token_contract")]))
}

pub fn update_tokens_and_distribution(
    deps: DepsMut,
    info: MessageInfo,
    lp_contract: String,
    reward_token: Vec<Denom>,
    distribution_schedule: Vec<Schedule>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&lp_contract)?;

    authcheck(deps.as_ref(), &info)?;

    if reward_token.len() != distribution_schedule.len() {
        return Err(ContractError::InvalidSchedules {});
    }

    config.reward_token = reward_token;
    config.lp_token_contract = lp_contract;
    config.distribution_schedule = distribution_schedule;

    CONFIG.save(deps.storage, &config)?;

    let mut state = STATE.load(deps.storage)?;

    while state.global_reward_index.len() < config.distribution_schedule.len() {
        state.global_reward_index.push(Decimal::zero());
    }

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![("action", "update_token_contract")]))
}

pub fn update_lock_duration(
    deps: DepsMut,
    info: MessageInfo,
    lock_duration: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    authcheck(deps.as_ref(), &info)?;
    config.lock_duration = lock_duration;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![("action", "update_lock_duration")]))
}

fn authcheck(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_new_schedules(
    config: &Config,
    state: &State,
    distribution_schedule: Vec<Schedule>,
) -> Result<(), ContractError> {
    if distribution_schedule.len() != config.distribution_schedule.len() {
        return Err(ContractError::InvalidSchedules {});
    }

    for (_index, schedule) in distribution_schedule.iter().enumerate() {
        let mut existing_counts: BTreeMap<Schedule, u32> = BTreeMap::new();
        let counter = existing_counts.entry(*schedule).or_insert(0);
        *counter += 1;

        let mut new_counts: BTreeMap<Schedule, u32> = BTreeMap::new();
        let counter = new_counts.entry(*schedule).or_insert(0);
        *counter += 1;

        for (schedule, count) in existing_counts.into_iter() {
            // if began ensure its in the new schedule
            if schedule.start_time <= state.last_distributed {
                if count > *new_counts.get(&schedule).unwrap_or(&0u32) {
                    return Err(ContractError::NewScheduleRemovePastDistribution {});
                }
                // after this new_counts will only contain the newly added schedules
                *new_counts.get_mut(&schedule).unwrap() -= count;
            }
        }

        for (schedule, count) in new_counts.into_iter() {
            if count > 0 && schedule.start_time <= state.last_distributed {
                return Err(ContractError::NewScheduleAddPastDistribution {});
            }
        }
    }

    Ok(())
}

// compute distributed rewards and update global reward index
pub fn compute_reward(config: &Config, state: &mut State, block_time: u64) {
    if state.total_bond_amount.is_zero() {
        state.last_distributed = block_time;
        return;
    }

    let mut distributed_amount = vec![Uint128::zero(); config.distribution_schedule.len()];

    for (index, schedule) in config.distribution_schedule.iter().enumerate() {
        if schedule.start_time > block_time || schedule.end_time < state.last_distributed {
            continue;
        }

        // min(s.1, block_time) - max(s.0, last_distributed)
        let passed_time = std::cmp::min(schedule.end_time, block_time)
            - std::cmp::max(schedule.start_time, state.last_distributed);

        let time = schedule.end_time - schedule.start_time;
        let distribution_amount_per_second: Decimal = Decimal::from_ratio(schedule.amount, time);
        distributed_amount[index] +=
            distribution_amount_per_second * Uint128::from(passed_time as u128);

        state.global_reward_index[index] +=
            Decimal::from_ratio(distributed_amount[index], state.total_bond_amount);
    }

    state.last_distributed = block_time;
}

// withdraw reward to pending reward
pub fn compute_staker_reward(state: &State, staker_info: &mut StakerInfo) -> StdResult<()> {
    while state.global_reward_index.len() > staker_info.reward_index.len() {
        staker_info.reward_index.push(Decimal::zero());
        staker_info.pending_reward.push(Uint128::zero());
    }

    for index in 0..staker_info.reward_index.len() {
        let pending_reward = (staker_info.bond_amount * state.global_reward_index[index])
            .checked_sub(staker_info.bond_amount * staker_info.reward_index[index])?;

        staker_info.reward_index[index] = state.global_reward_index[index];
        staker_info.pending_reward[index] += pending_reward;
    }
    Ok(())
}

fn increase_bond_amount(state: &mut State, staker_info: &mut StakerInfo, amount: Uint128) {
    state.total_bond_amount += amount;
    staker_info.bond_amount += amount;
}

fn decrease_bond_amount(
    state: &mut State,
    staker_info: &mut StakerInfo,
    amount: Uint128,
) -> StdResult<()> {
    state.total_bond_amount = state.total_bond_amount.checked_sub(amount)?;
    staker_info.bond_amount = staker_info.bond_amount.checked_sub(amount)?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }

    Ok(Response::default())
}
