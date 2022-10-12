use std::ops::Sub;

use cosmwasm_std::{
    coins, entry_point, to_binary, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use osmo_bindings::{OsmosisMsg, OsmosisQuery, Step, Swap, SwapAmountWithLimit};

use crate::error::WrapperError;
use crate::msg::{ExecuteMsg, InstantiateMsg};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response<OsmosisMsg>> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, WrapperError> {
    match msg {
        ExecuteMsg::Swap {
            user,
            first,
            route,
            amount,
            min_output,
            max_output,
        } => execute_swap(
            deps, env, info, user, first, route, amount, min_output, max_output,
        ),

        ExecuteMsg::CheckRange {
            user,
            denom,
            balance_before,
            min_output,
            max_output,
        } => execute_check_range(
            deps,
            env,
            info,
            user,
            denom,
            balance_before,
            min_output,
            max_output,
        ),
    }
}

pub fn execute_swap(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    _info: MessageInfo,
    user: String,
    first: Swap,
    route: Vec<Step>,
    amount: Uint128,
    min_output: Uint128,
    max_output: Uint128,
) -> Result<Response<OsmosisMsg>, WrapperError> {
    let mut msgs: Vec<CosmosMsg<OsmosisMsg>> = vec![];

    // Prepare swap message
    let swap = OsmosisMsg::Swap {
        first: first.clone(),
        amount: SwapAmountWithLimit::ExactIn {
            input: amount,
            min_output,
        },
        route: route.clone(),
    };
    msgs.push(swap.into());

    // Get output denom
    let last_denom = if !route.is_empty() {
        route[route.len() - 1].denom_out.clone()
    } else {
        first.denom_out
    };

    // Read current balance of the output asset
    let coin_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), last_denom.clone())?;

    // Add msg to check output amount
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::CheckRange {
            user,
            denom: last_denom,
            balance_before: coin_balance.amount,
            min_output,
            max_output,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_messages(msgs))
}

pub fn execute_check_range(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    user: String,
    denom: String,
    balance_before: Uint128,
    min_output: Uint128,
    max_output: Uint128,
) -> Result<Response<OsmosisMsg>, WrapperError> {
    // Validate this call
    if info.sender != env.contract.address {
        return Err(WrapperError::NotWrapperContract {
            expected: env.contract.address.into(),
            actual: info.sender.into(),
        });
    }

    // Query current balance
    let user_addr = deps.api.addr_validate(&user)?;
    let cur_balance = deps
        .querier
        .query_balance(env.contract.address, denom.clone())?;

    // Check if the output is in the range
    let output = cur_balance.amount.sub(balance_before);
    if output.lt(&min_output) || output.gt(&max_output) {
        return Err(WrapperError::InvalidOutput {
            expected_min: min_output,
            expected_max: max_output,
            actual: output,
        });
    }

    // Transfer output asset to the user
    let msgs: Vec<CosmosMsg<OsmosisMsg>> = vec![
        CosmosMsg::Bank(BankMsg::Send {
        to_address: user_addr.to_string(),
        amount: coins(output.u128(), denom),
    })];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(vec![("action", "execute_check_range")]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{coins, Coin, SystemResult};
    use cosmwasm_std::{OwnedDeps, SystemError};
    use std::marker::PhantomData;

    pub fn mock_dependencies(
        contract_balance: &[Coin],
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<OsmosisQuery>, OsmosisQuery> {
        let custom_querier: MockQuerier<OsmosisQuery> =
            MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]).with_custom_handler(|_| {
                SystemResult::Err(SystemError::InvalidRequest {
                    error: "not implemented".to_string(),
                    request: Default::default(),
                })
            });
        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: PhantomData,
        }
    }

    #[test]
    fn proper_instantialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
