use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("The sum of the ratios for dev wallets is not 1")]
    WrongRatio {},

    #[error("Invalid swap fee")]
    InvalidSwapFee {},

    #[error("Invalid burn ratio")]
    InvalidBurnRatio {},

    #[error("Pool already exists")]
    PoolExists {},
}