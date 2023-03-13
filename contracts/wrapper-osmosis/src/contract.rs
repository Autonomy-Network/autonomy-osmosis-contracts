use std::any;
use std::ops::Sub;

use cosmwasm_std::{
    coins, entry_point, to_binary, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use osmo_bindings::{OsmosisMsg, OsmosisQuery, Step, Swap, SwapAmountWithLimit};
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};
use osmosis_std::types::cosmos::base::v1beta1::{Coin as OsmoCoin};

use semver::Version;

use crate::error::WrapperError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "autonomy-wrapper-osmosis";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns a default object of type [`Response`] if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **_info** is an object of type [`MessageInfo`].
/// * **msg** is a message of type [`InstantiateMsg`] which contains the basic settings for creating the contract.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response<OsmosisMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

/// ## Description
/// Used for contract migration. Returns a default object of type [`Response`].
/// ## Params
/// * **_deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut<OsmosisQuery>, _env: Env, _msg: MigrateMsg) -> Result<Response, WrapperError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        // If state structure changed in any contract version in the way migration is needed, it
        // should occur here
    }
    Ok(Response::default())
}

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **msg** is an object of type [`ExecuteMsg`].
#[entry_point]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, WrapperError> {
    match msg {
        ExecuteMsg::Swap {
            sender,
            route,
            denom_in,
            amount_in,
            min_output,
            max_output,
            denom_out,
        } => execute_swap(
            deps, env, info, sender, route, amount_in, denom_in, min_output, max_output, denom_out
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

/// ## Description
/// Wrap osmosis swap operation between two assets. Returns [`WrapperError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **env** is an object of type [`Env`].
///
/// * **_info** is an object of type [`MessageInfo`].
///
/// * **user** is the address that receives the outputs.
///
/// * **first** is swap info for the first pool, `denom_in` is the input asset for the swap.
///
/// * **route** is route contains several pools connected to output asset.
///
/// * **amount** of input asset.
///
/// * **min_output** is minimum output amount.
///
/// * **max_output** is maximum output amount.
pub fn execute_swap(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    _info: MessageInfo,
    sender: String,
    route: Vec<SwapAmountInRoute>,
    amount_in: Uint128,
    denom_in: String,
    min_output: Uint128,
    max_output: Uint128,
    denom_out: String,
) -> Result<Response<OsmosisMsg>, WrapperError> {
    let mut msgs: Vec<CosmosMsg<OsmosisMsg>> = vec![];
    // Prepare swap message
    
    let swap = MsgSwapExactAmountIn {
        sender: sender.clone(),
        routes: route,
        token_in: Some(OsmoCoin{
            denom: denom_in,
            amount: amount_in.to_string(),
        }),
        token_out_min_amount: min_output.to_string(),
    };

    msgs.push(swap.into());

    // Read current balance of the output asset
    let coin_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), denom_out.clone())?;

    // Add msg to check output amount
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::CheckRange {
            user: sender.clone(),
            denom: denom_out,
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

/// ## Description
/// Validates swap output result. Returns [`WrapperError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **user** is the address that receives the outputs.
///
/// * **denom** of the output coin.
///
/// * **balance_before** is output token balance before swap.
///
/// * **min_output** is minimum output amount.
///
/// * **max_output** is maximum output amount.
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
    let msgs: Vec<CosmosMsg<OsmosisMsg>> = vec![CosmosMsg::Bank(BankMsg::Send {
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
    use cosmwasm_std::{coins, Coin, SystemResult, SubMsg, attr};
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

        // migrate
        let res = migrate(deps.as_mut(), mock_env(), MigrateMsg{}).unwrap();
        assert_eq!(res, Response::default());
    }

    #[test]
    fn test_execute_swap() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let info = mock_info("creator", &[]);
        // let first = Swap { pool_id: 1, denom_in: "in".to_string(), denom_out: "out".to_string() };
        let msg = ExecuteMsg::Swap {
            sender: "addr0".to_string(),
            route: vec![],
            denom_in: "earth".to_owned(),
            amount_in: Uint128::from(10u128),
            min_output: Uint128::from(10u128),
            max_output: Uint128::from(10u128),
            denom_out: "out".to_owned(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(MsgSwapExactAmountIn{
                    sender: "addr0".to_string(),
                    routes: vec![],
                    token_in: Some(OsmoCoin {
                        denom: "earth".to_owned(),
                        amount: Uint128::from(10u128).to_string(),
                    }),
                    token_out_min_amount: Uint128::from(10u128).to_string()
                }),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                    msg: to_binary(&ExecuteMsg::CheckRange {
                        user: "addr0".to_string(),
                        denom: "out".to_string(),
                        balance_before: Uint128::zero(),
                        min_output: Uint128::from(10u128),
                        max_output: Uint128::from(10u128),
                    }).unwrap(),
                    funds: vec![],
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "swap")
            ]
        )
    }

    #[test]
    fn test_check_range() {
        let mut deps = mock_dependencies(&coins(100u128, "earth"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::CheckRange {
            user: "addr0".to_string(),
            denom: "earth".to_string(),
            balance_before: Uint128::zero(),
            min_output: Uint128::from(10u128),
            max_output: Uint128::from(10u128),
        };

        // NotWrapperContract
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
        assert_eq!(err, Some(WrapperError::NotWrapperContract { expected: MOCK_CONTRACT_ADDR.to_string(), actual: "creator".to_string() }));

        // InvalidOutput
        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).err();
        assert_eq!(err, Some(WrapperError::InvalidOutput { expected_min: Uint128::from(10u128), expected_max: Uint128::from(10u128), actual: Uint128::from(100u128) }));

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        let msg = ExecuteMsg::CheckRange {
            user: "addr0".to_string(),
            denom: "earth".to_string(),
            balance_before: Uint128::zero(),
            min_output: Uint128::from(99u128),
            max_output: Uint128::from(101u128),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: "addr0".to_string(),
                        amount: coins(100u128, "earth"),
                    })
                )
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "execute_check_range")
            ]
        )
    }
}
