use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum CommonError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("All params should be available during instantiation")]
    InstantiateParamsUnavailable {},

    #[error("Incorrect number of addresses, expected {expected:?}, got {actual:?}")]
    AddressesQueryWrongNumber { expected: u32, actual: u32 },

    #[error("Invalid param: {param_name} is {invalid_value}, but it should be {predicate}")]
    InvalidParam {
        param_name: String,
        invalid_value: String,
        predicate: String,
    },
}

impl From<CommonError> for StdError {
    fn from(source: CommonError) -> Self {
        match source {
            CommonError::Std(e) => e,
            e => StdError::generic_err(format!("{e}")),
        }
    }
}
