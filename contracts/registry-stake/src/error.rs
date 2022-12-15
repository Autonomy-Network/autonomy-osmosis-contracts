use cw_controllers::AdminError;
use cw_utils::PaymentError;
use thiserror::Error;

use cosmwasm_std::{OverflowError, StdError};

use autonomy::error::CommonError;

/// ## Description
/// This enum describes registry contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Common(#[from] CommonError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("PaymnetError: {0}")]
    Payment(#[from] PaymentError),

    #[error("AUTO token or Stake amount can't be updated")]
    UpdateConfigError {},

    #[error("No executor")]
    NoExecutor {},

    #[error("Executor not updated")]
    ExecutorNotUpdated {},

    #[error("Invalid Executor")]
    InvalidExecutor {},

    #[error("Recurring requests can't have input assets")]
    NoInputAssetForRecurring {},

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

    #[error("Invalid recurring count")]
    InvalidRecurringCount {},

    #[error("Insufficient recurring fee")]
    InsufficientRecurringFee {},

    #[error("Target blacklisted")]
    TargetBlacklisted {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
