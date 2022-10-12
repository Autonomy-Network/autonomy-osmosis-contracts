use crate::asset::AssetInfo;

use cosmwasm_std::{
    to_binary, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Coin, QuerierWrapper,
    QueryRequest, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

// It's defined at https://github.com/terra-money/core/blob/d8e277626e74f9d6417dcd598574686882f0274c/types/assets/assets.go#L15
const NATIVE_TOKEN_PRECISION: u8 = 6;

/// Returns a native token's balance for a specific account.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **account_addr** is an object of type [`Addr`].
///
/// * **denom** is an object of type [`String`] used to specify the denomination used to return the balance (e.g uluna).
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

/// Returns the total balances for all coins at a specified account address.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **account_addr** is an object of type [`Addr`] which is the address for which we query balances.
pub fn query_all_balances(querier: &QuerierWrapper, account_addr: Addr) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: String::from(account_addr),
        }))?;
    Ok(all_balances.amount)
}

/// Returns a token balance for an account.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **contract_addr** is an object of type [`Addr`]. This is the token contract for which we return a balance.
///
/// * **account_addr** is an object of type [`Addr`] for which we query the token balance for.
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    // load balance from the token contract
    let res: Cw20BalanceResponse = querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&Cw20QueryMsg::Balance {
                address: String::from(account_addr),
            })?,
        }))
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(res.balance)
}

/// Returns a token's symbol.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **contract_addr** is an object of type [`Addr`] which is the token contract address.
pub fn query_token_symbol(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<String> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.symbol)
}

/// Returns the total supply of a specific token.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **contract_addr** is an object of type [`Addr`] which is the token contract address.
pub fn query_supply(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<Uint128> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// Returns the number of decimals that a token has.
/// ## Params
/// * **querier** is an object of type [`QuerierWrapper`].
///
/// * **asset_info** is an object of type [`AssetInfo`] and contains the asset details for a specific token.
pub fn query_token_precision(querier: &QuerierWrapper, asset_info: AssetInfo) -> StdResult<u8> {
    Ok(match asset_info {
        AssetInfo::NativeToken { denom: _ } => NATIVE_TOKEN_PRECISION,
        AssetInfo::Token { contract_addr } => {
            let res: TokenInfoResponse =
                querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;

            res.decimals
        }
    })
}
