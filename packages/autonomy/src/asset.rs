use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::querier::{query_balance, query_token_balance, query_token_symbol};
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, MessageInfo, QuerierWrapper, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

/// LUNA token denomination
pub const ULUNA_DENOM: &str = "uluna";

/// ## Description
/// This enum describes a Terra asset (native or CW20).
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Asset {
    /// Information about an asset stored in a [`AssetInfo`] struct
    pub info: AssetInfo,
    /// A token amount
    pub amount: Uint128,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

impl Asset {
    /// Returns true if the token is native. Otherwise returns false.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    /// Returns a message of type [`CosmosMsg`].
    ///
    /// For native tokens of type [`AssetInfo`] uses the default method [`BankMsg::Send`] to send a token amount to a recipient.
    ///
    /// For a token of type [`AssetInfo`] we use the default method [`Cw20ExecuteMsg::Transfer`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    ///
    /// * **recipient** is the address where the funds will be sent.
    pub fn into_msg(self, _querier: &QuerierWrapper, recipient: Addr) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken { denom } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![Coin {
                    denom: denom.to_string(),
                    amount,
                }],
            })),
        }
    }

    /// Validates an amount of native tokens being sent. Returns [`Ok`] if successful, otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **message_info** is an object of type [`MessageInfo`]
    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
            AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

impl AssetInfo {
    /// Returns true if the caller is a native token. Otherwise returns false.
    /// ## Params
    /// * **self** is the caller object type
    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { .. } => true,
            AssetInfo::Token { .. } => false,
        }
    }

    /// Returns the balance of token in a pool.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **pool_addr** is the address of the contract whose token balance we check.
    pub fn query_pool(&self, querier: &QuerierWrapper, pool_addr: Addr) -> StdResult<Uint128> {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                query_token_balance(querier, contract_addr.clone(), pool_addr)
            }
            AssetInfo::NativeToken { denom, .. } => {
                query_balance(querier, pool_addr, denom.to_string())
            }
        }
    }

    /// Returns True if the calling token is the same as the token specified in the input parameters.
    /// Otherwise returns False.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **asset** is object of type [`AssetInfo`].
    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token { contract_addr, .. } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken { .. } => false,
                }
            }
            AssetInfo::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token { .. } => false,
                    AssetInfo::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }

    /// If the caller object is a native token of type ['AssetInfo`] then his `denom` field converts to a byte string.
    ///
    /// If the caller object is a token of type ['AssetInfo`] then his `contract_addr` field converts to a byte string.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfo::NativeToken { denom } => denom.as_bytes(),
            AssetInfo::Token { contract_addr } => contract_addr.as_bytes(),
        }
    }

    /// Returns [`Ok`] if the token of type [`AssetInfo`] is in lowercase and valid. Otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **api** is a object of type [`Api`]
    pub fn check(&self, api: &dyn Api) -> StdResult<()> {
        match self {
            AssetInfo::Token { contract_addr } => {
                addr_validate_to_lower(api, contract_addr.as_str())?;
            }
            AssetInfo::NativeToken { denom } => {
                if !denom.starts_with("ibc/") && denom != &denom.to_lowercase() {
                    return Err(StdError::generic_err(format!(
                        "Non-IBC token denom {} should be lowercase",
                        denom
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Returns a lowercased, validated address upon success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is an object of type [`Api`]
///
/// * **addr** is an object of type [`Addr`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(addr)
}

const TOKEN_SYMBOL_MAX_LENGTH: usize = 4;

/// Returns a formatted LP token name
/// ## Params
/// * **asset_infos** is an array with two items the type of [`AssetInfo`].
///
/// * **querier** is an object of type [`QuerierWrapper`].
pub fn format_lp_token_name(
    asset_infos: [AssetInfo; 2],
    querier: &QuerierWrapper,
) -> StdResult<String> {
    let mut short_symbols: Vec<String> = vec![];
    for asset_info in asset_infos {
        let short_symbol = match asset_info {
            AssetInfo::NativeToken { denom } => {
                denom.chars().take(TOKEN_SYMBOL_MAX_LENGTH).collect()
            }
            AssetInfo::Token { contract_addr } => {
                let token_symbol = query_token_symbol(querier, contract_addr)?;
                token_symbol.chars().take(TOKEN_SYMBOL_MAX_LENGTH).collect()
            }
        };
        short_symbols.push(short_symbol);
    }
    Ok(format!("{}-{}-LP", short_symbols[0], short_symbols[1]).to_uppercase())
}

/// Returns an [`Asset`] object representing a native token and an amount of tokens.
/// ## Params
/// * **denom** is a [`String`] that represents the native asset denomination.
///
/// * **amount** is a [`Uint128`] representing an amount of native assets.
pub fn native_asset(denom: String, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::NativeToken { denom },
        amount,
    }
}

/// Returns an [`Asset`] object representing a non-native token and an amount of tokens.
/// ## Params
/// * **contract_addr** is a [`Addr`]. It is the address of the token contract.
///
/// * **amount** is a [`Uint128`] representing an amount of tokens.
pub fn token_asset(contract_addr: Addr, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::Token { contract_addr },
        amount,
    }
}

/// Returns an [`AssetInfo`] object representing the denomination for a Terra native asset.
/// ## Params
/// * **denom** is a [`String`] object representing the denomination of the Terra native asset.
pub fn native_asset_info(denom: String) -> AssetInfo {
    AssetInfo::NativeToken { denom }
}

/// Returns an [`AssetInfo`] object representing the address of a token contract.
/// ## Params
/// * **contract_addr** is a [`Addr`] object representing the address of a token contract.
pub fn token_asset_info(contract_addr: Addr) -> AssetInfo {
    AssetInfo::Token { contract_addr }
}

#[cfg(test)]
mod test {
    use super::super::testing::mock_dependencies;
    use super::*;

    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::{to_binary, Addr, BankMsg, Coin, CosmosMsg, Uint128, WasmMsg};
    use cw20::Cw20ExecuteMsg;

    #[test]
    fn test_asset_info() {
        let token_info: AssetInfo = AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        };
        let native_token_info: AssetInfo = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        assert!(!token_info.equal(&native_token_info));

        assert!(
            !token_info.equal(&AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            })
        );

        assert!(
            token_info.equal(&AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            })
        );

        assert!(native_token_info.is_native_token());
        assert!(!token_info.is_native_token());

        let mut deps = mock_dependencies();

        deps.querier.set_cw20_balance(
            &String::from("asset0000"),
            &String::from(MOCK_CONTRACT_ADDR),
            123u128,
        );
        deps.querier.set_base_balances(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin::new(123u128, "uusd")],
        );

        assert_eq!(
            native_token_info
                .query_pool(&deps.as_ref().querier, Addr::unchecked(MOCK_CONTRACT_ADDR))
                .unwrap(),
            Uint128::new(123u128)
        );
        assert_eq!(
            token_info
                .query_pool(&deps.as_ref().querier, Addr::unchecked(MOCK_CONTRACT_ADDR))
                .unwrap(),
            Uint128::new(123u128)
        );
    }

    #[test]
    fn test_asset() {
        let mut deps = mock_dependencies();

        deps.querier.set_cw20_balance(
            &String::from("asset0000"),
            &String::from(MOCK_CONTRACT_ADDR),
            123u128,
        );
        deps.querier.set_cw20_balance(
            &String::from("asset0000"),
            &String::from("addr00000"),
            123u128,
        );
        deps.querier.set_base_balances(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin::new(123123u128, "uusd")],
        );
        deps.querier
            .set_base_balances(&String::from("addr00000"), &[Coin::new(123123u128, "uusd")]);

        let token_asset = Asset {
            amount: Uint128::new(123123u128),
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
        };
        assert!(!token_asset.is_native_token());

        let native_token_asset = Asset {
            amount: Uint128::new(123123u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        };
        assert!(native_token_asset.is_native_token());

        assert_eq!(
            token_asset
                .into_msg(&deps.as_ref().querier, Addr::unchecked("addr0000"))
                .unwrap(),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("asset0000"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("addr0000"),
                    amount: Uint128::new(123123u128),
                })
                .unwrap(),
                funds: vec![],
            })
        );

        assert_eq!(
            native_token_asset
                .into_msg(&deps.as_ref().querier, Addr::unchecked("addr0000"))
                .unwrap(),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::new(123123u128),
                }]
            })
        );
    }

    #[test]
    fn creating_instances() {
        let info = AssetInfo::Token {
            contract_addr: Addr::unchecked("mock_token"),
        };
        assert_eq!(
            info,
            AssetInfo::Token {
                contract_addr: Addr::unchecked("mock_token")
            }
        );

        let info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };
        assert_eq!(
            info,
            AssetInfo::NativeToken {
                denom: String::from("uusd")
            }
        );
    }

    #[test]
    fn comparing() {
        let uluna = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };
        let uusd = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };
        let astro = AssetInfo::Token {
            contract_addr: Addr::unchecked("astro_token"),
        };
        let mars = AssetInfo::Token {
            contract_addr: Addr::unchecked("mars_token"),
        };

        assert!(uluna != uusd);
        assert!(uluna != astro);
        assert!(astro != mars);
        assert!(uluna == uluna.clone());
        assert!(astro == astro.clone());
    }

    #[test]
    fn to_string() {
        let info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };
        assert_eq!(info.to_string(), String::from("uusd"));

        let info = AssetInfo::Token {
            contract_addr: Addr::unchecked("mock_token"),
        };
        assert_eq!(info.to_string(), String::from("mock_token"));

        let asset = Asset {
            amount: Uint128::new(123u128),
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mock_token"),
            },
        };
        assert_eq!(asset.to_string(), String::from("123mock_token"));

        let asset = Asset {
            amount: Uint128::new(123u128),
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        };
        assert_eq!(asset.to_string(), String::from("123uusd"));
    }
}
