use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum WrapperError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Permission denied: the sender must be the wrapper")]
    NotWrapperContract { expected: String, actual: String },

    #[error("Invalid output amount")]
    InvalidOutput {
        expected_min: Uint128,
        expected_max: Uint128,
        actual: Uint128,
    },
}

impl From<semver::Error> for WrapperError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
