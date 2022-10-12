#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, SubMsgResult, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use autonomy::asset::{Asset, AssetInfo};
use autonomy::error::CommonError;
use autonomy::helper::{option_string_to_addr, zero_address, zero_string};
use autonomy::types::OrderBy;

use crate::error::ContractError;
use crate::msg::{
    CreateOrUpdateConfig, CreateRequestInfo, Cw20HookMsg, EpochInfoResponse,
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RequestInfoResponse, RequestsResponse,
    StakeAmountResponse, StakesResponse, StateResponse,
};
use crate::state::{
    read_balance, read_config, read_request, read_requests, read_state, remove_request,
    store_balance, store_config, store_request, store_state, Config, Request, State,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        fee_amount,
        fee_denom,
        auto,
        stake_amount,
        blocks_in_epoch,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && fee_amount.is_some()
        && fee_denom.is_some()
        && auto.is_some()
        && stake_amount.is_some()
        && blocks_in_epoch.is_some();

    if !available {
        return Err(CommonError::InstantiateParamsUnavailable {}.into());
    }

    let config = Config {
        owner: option_string_to_addr(deps.api, owner, zero_address())?,
        fee_amount: fee_amount.unwrap(),
        fee_denom: fee_denom.unwrap(),
        auto: auto.unwrap(),
        stake_amount: stake_amount.unwrap(),
        blocks_in_epoch: blocks_in_epoch.unwrap(),
    };

    let state = State {
        curr_executing_request_id: u64::MAX,
        next_request_id: 0,
        last_epoch: 0,
        total_requests: 0,
        executor: zero_string(),
        stakes: vec![],
        total_staked: Uint128::zero(),
    };

    store_config(deps.storage, &config)?;
    store_state(deps.storage, &state)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),

        ExecuteMsg::CreateRequest { request_info } => create_request(deps, env, info, request_info),

        ExecuteMsg::CancelRequest { id } => cancel_request(deps, env, info, id),

        ExecuteMsg::ExecuteRequest { id } => execute_request(deps, info, id),

        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),

        ExecuteMsg::StakeDenom { num_stakes } => receive_denom(deps, env, info, num_stakes),

        ExecuteMsg::Unstake { idxs } => unstake(deps, env, info, idxs),

        ExecuteMsg::UpdateExecutor {} => update_executor(deps, env),
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = read_config(deps.storage)?;

    if info.sender != config.owner {
        return Err(CommonError::Unauthorized {}.into());
    }

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        fee_amount,
        fee_denom,
        auto,
        stake_amount,
        blocks_in_epoch,
    } = new_config;

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.fee_amount = fee_amount.unwrap_or(config.fee_amount);
    config.fee_denom = fee_denom.unwrap_or(config.fee_denom);
    config.auto = auto.unwrap_or(config.auto);
    config.stake_amount = stake_amount.unwrap_or(config.stake_amount);
    config.blocks_in_epoch = blocks_in_epoch.unwrap_or(config.blocks_in_epoch);

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn create_request(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_info: CreateRequestInfo,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;
    let mut state = read_state(deps.storage)?;

    let target_addr = deps.api.addr_validate(&request_info.target)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut funds = info.funds.clone();

    // Funds can't be empty
    if funds.is_empty() {
        return Err(ContractError::InvalidInputAssets {});
    }

    // Funds should contain execution fee
    if let Some(fee_fund_index) = funds.iter().position(|f| f.denom == config.fee_denom) {
        // Fee amount should be enough
        if funds[fee_fund_index].amount < config.fee_amount {
            return Err(ContractError::InsufficientFee {});
        }

        // Subtract fee amount
        funds[fee_fund_index].amount -= config.fee_amount;
    } else {
        return Err(ContractError::NoFeePaid {});
    }

    // Check fund tokens will be used for request
    match request_info.input_asset.clone().info {
        AssetInfo::NativeToken { denom } => {
            if let Some(asset_index) = funds.iter().position(|f| f.denom == denom) {
                // Check if actual amount matches with amount passed by params
                if funds[asset_index].amount < request_info.input_asset.amount {
                    return Err(ContractError::InvalidInputAssets {});
                }
            } else {
                return Err(ContractError::InvalidInputAssets {});
            }
        }
        AssetInfo::Token { contract_addr } => {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: request_info.input_asset.amount,
                })?,
                funds: vec![],
            }));
        }
    }

    // Update executor
    let cur_epoch = env.block.height / config.blocks_in_epoch * config.blocks_in_epoch;
    if cur_epoch != state.last_epoch {
        _update_executor(&mut state, env.clone(), config.blocks_in_epoch);

        // if state.executor == "" {
        //     return Err(StdError::generic_err("no executor"));
        // }
    }

    // Create and save request struct
    let id = state.next_request_id;
    let request = Request {
        user: info.sender.to_string(),
        executor: state.executor.to_string(),
        target: target_addr.to_string(),
        msg: request_info.msg,
        input_asset: request_info.input_asset,
        created_at: env.block.time.seconds(),
    };

    state.next_request_id += 1;
    state.total_requests += 1;

    store_request(deps.storage, id, &request)?;
    store_state(deps.storage, &state)?;

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "create_request"),
        attr("id", id.to_string()),
    ]))
}

pub fn cancel_request(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    let request = read_request(deps.storage, id)?;

    // Validate owner
    let request_owner = deps.api.addr_validate(request.user.as_str())?;
    if request_owner != info.sender {
        return Err(CommonError::Unauthorized {}.into());
    }

    // Returun escrowed tokens
    let mut msgs: Vec<CosmosMsg> = vec![];

    let input_asset = request.input_asset.clone();
    match input_asset.info {
        AssetInfo::NativeToken { denom: _ } => {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: request.user.to_string(),
                amount: vec![input_asset.deduct_tax(&deps.querier)?],
            }));
        }
        AssetInfo::Token { contract_addr } => {
            if !request.input_asset.amount.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: request.user.to_string(),
                        amount: request.input_asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    // Return fee asset
    let fee_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.fee_denom,
        },
        amount: config.fee_amount,
    };
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: request.user,
        amount: vec![fee_asset.deduct_tax(&deps.querier)?],
    }));

    // Remove request
    let mut state = read_state(deps.storage)?;
    state.total_requests -= 1;
    store_state(deps.storage, &state)?;

    remove_request(deps.storage, id)?;

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "cancel_request"),
        attr("id", id.to_string()),
    ]))
}

pub fn execute_request(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    let request = read_request(deps.storage, id)?;

    // Validate executor
    if !request.executor.is_empty() {
        let executor = deps.api.addr_validate(&request.executor)?;
        if executor != info.sender {
            return Err(CommonError::Unauthorized {}.into());
        }
    }

    // Update current executing request id
    let mut state = read_state(deps.storage)?;
    state.curr_executing_request_id = id;
    store_state(deps.storage, &state)?;

    // Forward escrowed assets and execute contract
    let mut msgs = vec![];

    let input_asset = request.input_asset.clone();
    match input_asset.info {
        AssetInfo::NativeToken { denom: _ } => {
            msgs.push(SubMsg {
                id: 1,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: request.target.to_string(),
                    funds: vec![input_asset.deduct_tax(&deps.querier)?],
                    msg: request.msg,
                }),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            });
        }
        AssetInfo::Token { contract_addr } => {
            if !request.input_asset.amount.is_zero() {
                msgs.push(SubMsg {
                    id: 0,
                    msg: CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: request.target.to_string(),
                            amount: request.input_asset.amount,
                        })?,
                        funds: vec![],
                    }),
                    gas_limit: None,
                    reply_on: ReplyOn::Never,
                });
            }
            msgs.push(SubMsg {
                id: 1,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: request.target.to_string(),
                    funds: vec![],
                    msg: request.msg,
                }),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            });
        }
    }

    // Transfer fee to executor
    let fee_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.fee_denom,
        },
        amount: config.fee_amount,
    };
    msgs.push(SubMsg {
        id: 0,
        msg: CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![fee_asset.deduct_tax(&deps.querier)?],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Never,
    });

    // Remove request
    let mut state = read_state(deps.storage)?;
    state.total_requests -= 1;
    store_state(deps.storage, &state)?;

    remove_request(deps.storage, id)?;

    Ok(Response::new().add_submessages(msgs).add_attributes(vec![
        attr("action", "execute_request"),
        attr("id", id.to_string()),
    ]))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Stake { num_stakes }) => {
            match config.auto {
                AssetInfo::Token { contract_addr } => {
                    // only AUTO token contract can execute this message
                    if contract_addr != info.sender {
                        return Err(CommonError::Unauthorized {}.into());
                    }
                }
                AssetInfo::NativeToken { denom: _ } => {
                    return Err(ContractError::InvalidAutoToken {});
                }
            }

            let cw20_sender = deps.api.addr_validate(&cw20_msg.sender)?;
            stake(deps, env, info, &cw20_sender, num_stakes, cw20_msg.amount)
        }
        Err(_) => Err(ContractError::DataShouldBeGiven {}),
    }
}

pub fn receive_denom(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    num_stakes: u64,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    match config.auto {
        AssetInfo::Token { contract_addr: _ } => {
            Err(CommonError::Unauthorized { }.into())
        }
        AssetInfo::NativeToken { denom } => {
            let received_auto = info
                .funds
                .iter()
                .find(|c| c.denom == denom)
                .map(|c| c.amount)
                .unwrap_or(Uint128::zero());
            let staker = info.clone().sender;
            stake(deps, env, info, &staker, num_stakes, received_auto)
        }
    }
}

pub fn stake(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: &Addr,
    num_stakes: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    // Validate amount
    if config.stake_amount * Uint128::from(num_stakes) != amount {
        return Err(ContractError::InvalidStakeInfo { });
    }

    // Update executor
    let mut state = read_state(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    store_state(deps.storage, &state)?;

    // Update stakes array
    for _ in 0..num_stakes {
        state.stakes.push(sender.to_string());
    }

    // Add amount to stake balance
    let balance = read_balance(deps.storage, sender.clone()) + amount;
    store_balance(deps.storage, sender.clone(), &balance)?;
    state.total_staked += amount;
    store_state(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "stake"),
        attr("user", sender),
        attr("num_stakes", num_stakes.to_string()),
    ]))
}

pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    idxs: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    // Update executor
    let mut state = read_state(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    store_state(deps.storage, &state)?;

    // Validate and remove stakes
    for idx in &idxs {
        let idx = *idx as usize;
        let addr = deps.api.addr_validate(&state.stakes[idx])?;
        if addr != info.sender {
            return Err(ContractError::IdxNotYou { });
        }
        if idx >= state.stakes.len() {
            return Err(ContractError::IdxOutOfBound { });
        }
        state.stakes.swap_remove(idx);
    }

    // Update stake balance
    let amount = Uint128::from(idxs.len() as u64) * config.stake_amount;
    let balance = read_balance(deps.storage, info.sender.clone()) - amount;
    store_balance(deps.storage, info.sender.clone(), &balance)?;
    state.total_staked -= amount;
    store_state(deps.storage, &state)?;

    // Return assets
    let mut msgs: Vec<CosmosMsg> = vec![];
    match config.auto {
        AssetInfo::Token { contract_addr: _ } => {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.auto.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount,
                })?,
                funds: vec![],
            }));
        }
        AssetInfo::NativeToken { denom } => {
            let asset = Asset {
                info: AssetInfo::NativeToken { denom },
                amount,
            };
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![asset.deduct_tax(&deps.querier)?],
            }));
        }
    }

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unstake"),
        attr("user", info.sender),
        attr("count", idxs.len().to_string()),
    ]))
}

fn _update_executor(state: &mut State, env: Env, blocks_in_epoch: u64) {
    let last_epoch = env.block.height / blocks_in_epoch * blocks_in_epoch;
    if state.last_epoch != last_epoch {
        let len = state.stakes.len() as u64;

        if len > 0 {
            let mut rng = oorandom::Rand64::new(env.block.height as u128);
            let index = rng.rand_u64() % len;
            state.executor = state.stakes[index as usize].clone();
            state.last_epoch = last_epoch;
        } else {
            state.executor = zero_string();
        }
    }
}

pub fn update_executor(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;

    let mut state = read_state(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    store_state(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_executor"),
        attr("epoch", state.last_epoch.to_string()),
        attr("executor", state.executor),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => execute_reply(deps, env, msg.result),
        _ => Err(CommonError::Unauthorized {}.into()),
    }
}

pub fn execute_reply(
    deps: DepsMut,
    _env: Env,
    _msg: SubMsgResult,
) -> Result<Response, ContractError> {
    let mut state = read_state(deps.storage)?;
    state.curr_executing_request_id = u64::MAX;
    store_state(deps.storage, &state)?;
    Ok(Response::new().add_attributes(vec![attr("action", "finialize_execute")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),

        QueryMsg::RequestInfo { id } => Ok(to_binary(&query_request_info(deps, id)?)?),

        QueryMsg::Requests {
            start_after,
            limit,
            order_by,
        } => Ok(to_binary(&query_requests(
            deps,
            start_after,
            limit,
            order_by,
        )?)?),

        QueryMsg::State {} => Ok(to_binary(&query_state(deps)?)?),

        QueryMsg::EpochInfo {} => Ok(to_binary(&query_epoch_info(deps, env)?)?),

        QueryMsg::StakeAmount { user } => Ok(to_binary(&query_stake_amount(deps, user)?)?),

        QueryMsg::Stakes { start, limit } => Ok(to_binary(&query_stakes(deps, start, limit)?)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = read_config(deps.storage)?;
    Ok(config)
}

pub fn query_request_info(deps: Deps, id: u64) -> StdResult<RequestInfoResponse> {
    let info = read_request(deps.storage, id).unwrap_or(Request {
        user: zero_string(),
        executor: zero_string(),
        target: zero_string(),
        msg: to_binary("")?,
        input_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::zero(),
        },
        created_at: 0,
    });
    Ok(RequestInfoResponse { id, request: info })
}

pub fn query_requests(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<RequestsResponse> {
    let requests = if let Some(start_after) = start_after {
        read_requests(deps.storage, Some(start_after), limit, order_by)?
    } else {
        read_requests(deps.storage, None, limit, order_by)?
    };

    let requests_responses: StdResult<Vec<RequestInfoResponse>> = requests
        .iter()
        .map(|request| {
            Ok(RequestInfoResponse {
                id: request.0,
                request: request.1.clone(),
            })
        })
        .collect();

    Ok(RequestsResponse {
        requests: requests_responses?,
    })
}

pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = read_state(deps.storage)?;
    let resp = StateResponse {
        curr_executing_request_id: state.curr_executing_request_id,
        total_requests: state.total_requests,
        next_request_id: state.next_request_id,
        total_stake_amount: state.total_staked,
        stakes_len: state.stakes.len() as u64,
    };

    Ok(resp)
}

pub fn query_epoch_info(deps: Deps, env: Env) -> StdResult<EpochInfoResponse> {
    let state = read_state(deps.storage)?;
    let config = read_config(deps.storage)?;
    let cur_epoch = env.block.height / config.blocks_in_epoch * config.blocks_in_epoch;
    let resp = EpochInfoResponse {
        cur_epoch,
        last_epoch: state.last_epoch,
        executor: state.executor,
    };

    Ok(resp)
}

pub fn query_stake_amount(deps: Deps, user: String) -> StdResult<StakeAmountResponse> {
    let amount = read_balance(deps.storage, deps.api.addr_validate(&user)?);
    let resp = StakeAmountResponse { amount };

    Ok(resp)
}

pub fn query_stakes(deps: Deps, start: u64, limit: u64) -> StdResult<StakesResponse> {
    let state = read_state(deps.storage)?;

    let mut end = (start + limit) as usize;
    if end > state.stakes.len() {
        end = state.stakes.len()
    };
    let start = start as usize;

    Ok(StakesResponse {
        stakes: state.stakes[start..end].to_vec(),
    })
}
