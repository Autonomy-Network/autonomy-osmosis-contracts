use thiserror::Error;

use cosmwasm_std::{OverflowError, StdError};

use autonomy::error::CommonError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Common(#[from] CommonError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("No executor")]
    NoExecutor {},

    #[error("Invalid input assets")]
    InvalidInputAssets {},

    #[error("AUTO token invalid")]
    InvalidAutoToken {},

    #[error("Insufficient fees")]
    InsufficientFee {},

    #[error("No fee paid")]
    NoFeePaid {},

    #[error("Data should be given")]
    DataShouldBeGiven {},

    #[error("Invalid Stake Info")]
    InvalidStakeInfo {},

    #[error("Idx not yours")]
    IdxNotYou {},

    #[error("Idx is out of bound")]
    IdxOutOfBound {},
}
