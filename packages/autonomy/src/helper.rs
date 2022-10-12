use cosmwasm_std::{
    Addr, Api, StdResult,
};

/// Used when unwrapping an optional address sent in a contract call by a user.
/// Validates addreess if present, otherwise uses a given default value.
pub fn option_string_to_addr(
    api: &dyn Api,
    option_string: Option<String>,
    default: Addr,
) -> StdResult<Addr> {
    match option_string {
        Some(input_addr) => api.addr_validate(&input_addr),
        None => Ok(default),
    }
}

pub fn zero_address() -> Addr {
    Addr::unchecked("")
}

pub fn zero_string() -> String {
    "".to_string()
}
