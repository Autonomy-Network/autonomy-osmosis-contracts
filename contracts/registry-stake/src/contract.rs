#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdResult, SubMsg, SubMsgResult, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use autonomy::asset::{Asset, AssetInfo};
use autonomy::error::CommonError;
use autonomy::helper::zero_string;
use autonomy::types::OrderBy;
use cw_utils::must_pay;
use semver::Version;

use crate::error::ContractError;
use crate::msg::{
    BlacklistResponse, CreateOrUpdateConfig, CreateRequestInfo, Cw20HookMsg, EpochInfoResponse,
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RecurringFeeAmountResponse,
    RequestInfoResponse, RequestsResponse, StakeAmountResponse, StakesResponse, StateResponse,
};
use crate::state::{
    read_requests, Config, Request, State, ADMIN, BLACKLIST, CONFIG, NEW_ADMIN, RECURRING_BALANCE,
    REQUESTS, STAKE_BALANCE, STATE,
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "autonomy-registry-stake";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns a default object of type [`Response`] if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **_info** is an object of type [`MessageInfo`].
/// * **msg** is a message of type [`InstantiateMsg`] which contains the basic settings for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        admin,
        fee_amount,
        fee_denom,
        auto,
        stake_amount,
        blocks_in_epoch,
    } = msg.config;

    // All fields should be available
    let available = admin.is_some()
        && fee_amount.is_some()
        && fee_denom.is_some()
        && auto.is_some()
        && stake_amount.is_some()
        && blocks_in_epoch.is_some();

    if !available {
        return Err(CommonError::InstantiateParamsUnavailable {}.into());
    }

    // Validator AUTO token
    let auto = auto.unwrap();
    auto.check(deps.api)?;

    let admin_addr = admin
        .map(|admin| deps.api.addr_validate(&admin))
        .transpose()?;
    ADMIN.set(deps.branch(), admin_addr)?;

    let config = Config {
        fee_amount: fee_amount.unwrap(),
        fee_denom: fee_denom.unwrap(),
        auto,
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
        total_recurring_fee: Uint128::zero(),
    };

    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

/// ## Description
/// Used for contract migration. Returns a default object of type [`Response`].
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ClaimAdmin {} => claim_admin(deps, env, info),
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),

        // Registry
        ExecuteMsg::CreateRequest { request_info } => create_request(deps, env, info, request_info),
        ExecuteMsg::CancelRequest { id } => cancel_request(deps, env, info, id),
        ExecuteMsg::ExecuteRequest { id } => execute_request(deps, env, info, id),
        ExecuteMsg::DepositRecurringFee { recurring_count } => {
            deposit_recurring_fee(deps, info, recurring_count)
        }
        ExecuteMsg::WithdrawRecurringFee { recurring_count } => {
            withdraw_recurring_fee(deps, info, recurring_count)
        }

        // Staking
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::StakeDenom { num_stakes } => receive_denom(deps, env, info, num_stakes),
        ExecuteMsg::Unstake { idxs } => unstake(deps, env, info, idxs),
        ExecuteMsg::UpdateExecutor {} => update_executor(deps, env),

        // Blacklist
        ExecuteMsg::AddToBlacklist { addrs } => add_to_blacklist(deps, env, info, addrs),
        ExecuteMsg::RemoveFromBlacklist { addrs } => remove_from_blacklist(deps, env, info, addrs),
    }
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **new_config** is an object of type [`CreateOrUpdateConfig`] that contains the parameters to update.
///
/// ## Executor
/// Only the admin can execute this.
pub fn update_config(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only admin can update config
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        admin,
        fee_amount,
        fee_denom,
        auto,
        stake_amount,
        blocks_in_epoch,
    } = new_config;

    if auto.is_some() || stake_amount.is_some() {
        return Err(ContractError::UpdateConfigError {});
    }

    let admin_addr = admin
        .map(|admin| deps.api.addr_validate(&admin))
        .transpose()?;
    NEW_ADMIN.set(deps.branch(), admin_addr)?;

    config.fee_amount = fee_amount.unwrap_or(config.fee_amount);
    config.fee_denom = fee_denom.unwrap_or(config.fee_denom);
    config.blocks_in_epoch = blocks_in_epoch.unwrap_or(config.blocks_in_epoch);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// ## Description
/// Take the admin permission of the contract. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// ## Executor
/// Only the proposed admin can execute this.
pub fn claim_admin(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Only admin can update config
    NEW_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    ADMIN.set(deps.branch(), Some(info.sender.clone()))?;
    NEW_ADMIN.set(deps.branch(), None)?;

    Ok(Response::new()
        .add_attribute("action", "claim_admin")
        .add_attribute("new admin", info.sender))
}

/// ## Description
/// Creates a new request
/// * Funds should cover the execution fee and the asset for the request execution
/// * Executor for the current epoch is set for this request
///   if there's no executor, anyone can execute the request
/// * Request Id increases from zero by one
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **request_info** is an object of type [`CreateRequestInfo`].
pub fn create_request(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_info: CreateRequestInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let target_addr = deps.api.addr_validate(&request_info.target)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut funds = info.funds.clone();

    // Check if blacklisted
    if BLACKLIST.has(deps.storage, &target_addr) {
        return Err(ContractError::TargetBlacklisted {});
    }

    // Recurring requests can't have input assets
    if request_info.is_recurring && request_info.input_asset != None {
        return Err(ContractError::NoInputAssetForRecurring {});
    }

    // If this is not recurring request, funds should contain execution fee
    if !request_info.is_recurring {
        if let Some(fee_fund_index) = funds.iter().position(|f| f.denom == config.fee_denom) {
            // Fee amount should be enough
            if funds[fee_fund_index].amount < config.fee_amount {
                return Err(ContractError::InsufficientFee {});
            }

            // Funds array is used for the input asset process
            // so Subtract fee amount
            funds[fee_fund_index].amount -= config.fee_amount;
        } else {
            return Err(ContractError::NoFeePaid {});
        }
    }

    // Check fund tokens will be used for request
    if let Some(input_asset) = request_info.input_asset.clone() {
        match input_asset.info {
            AssetInfo::NativeToken { denom } => {
                if let Some(asset_index) = funds.iter().position(|f| f.denom == denom) {
                    // Check if actual amount matches with amount passed by params
                    if funds[asset_index].amount < input_asset.amount {
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
                        amount: input_asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    // Create and save request struct
    let id = state.next_request_id;
    let request = Request {
        user: info.sender.to_string(),
        target: target_addr.to_string(),
        msg: request_info.msg,
        input_asset: request_info.input_asset,
        is_recurring: request_info.is_recurring,
        created_at: env.block.time.seconds(),
    };

    state.next_request_id += 1;
    state.total_requests += 1;

    REQUESTS.save(deps.storage, id, &request)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "create_request"),
        attr("id", id.to_string()),
        attr("user", request.user),
        attr("target", request.target),
        attr("msg", request.msg.to_string()),
        attr("asset", format!("{:?}", request.input_asset)),
        attr(
            "is_recurring",
            if request.is_recurring {
                "true"
            } else {
                "false"
            },
        ),
        attr("created_at", request.created_at.to_string()),
    ]))
}

/// ## Description
/// Cancel the request with [`id`]. Returns a [`ContractError`] on failure.
/// * Return the escrowed assets for the request execution.
/// * Return execution fee.
/// * Remove request from the storage.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **id** is the request id, which an object of type [`u64`].
///
/// ## Executor
/// Only the owner of the request can execute this.
pub fn cancel_request(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let request = REQUESTS.load(deps.storage, id)?;

    // Validate owner
    let request_owner = deps.api.addr_validate(request.user.as_str())?;
    if request_owner != info.sender {
        return Err(CommonError::Unauthorized {}.into());
    }

    // Returun escrowed tokens
    let recipient = deps.api.addr_validate(&request.user)?;
    let mut msgs: Vec<CosmosMsg> = vec![];

    if let Some(input_asset) = request.input_asset {
        msgs.push(input_asset.into_msg(&deps.querier, recipient.clone())?);
    }

    // Return fee asset if not recurring request
    if !request.is_recurring {
        let fee_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.fee_denom,
            },
            amount: config.fee_amount,
        };
        msgs.push(fee_asset.into_msg(&deps.querier, recipient)?);
    }

    // Remove request
    let mut state = STATE.load(deps.storage)?;
    state.total_requests -= 1;
    STATE.save(deps.storage, &state)?;

    REQUESTS.remove(deps.storage, id);

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "cancel_request"),
        attr("id", id.to_string()),
    ]))
}

/// ## Description
/// Execute request with [`id`]. Returns a [`ContractError`] on failure.
/// * Forward escrowed assets and call the target contract.
/// * Transfer execution fees to the executor.
/// * Fails if executor doesn't match.
/// * Request remains if it's recurring.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **id** is the request id, which an object of type [`u64`].
///
/// ## Executor
/// Only the excutor of the current epoch can execute this.
pub fn execute_request(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let request = REQUESTS.load(deps.storage, id)?;
    let mut state = STATE.load(deps.storage)?;

    // Validate executor
    let cur_epoch = env.block.height / config.blocks_in_epoch * config.blocks_in_epoch;
    if cur_epoch != state.last_epoch {
        return Err(ContractError::ExecutorNotUpdated {});
    }

    if !state.executor.is_empty() {
        let executor = deps.api.addr_validate(&state.executor)?;
        if executor != info.sender {
            return Err(ContractError::InvalidExecutor {});
        }
    }

    // If recurring request, deduct fee from the pool
    if request.is_recurring {
        let user = deps.api.addr_validate(&request.user)?;
        let mut balance = RECURRING_BALANCE
            .load(deps.storage, &user)
            .unwrap_or_default();
        if balance < config.fee_amount {
            return Err(ContractError::InsufficientRecurringFee {});
        }
        balance -= config.fee_amount;
        state.total_recurring_fee -= config.fee_amount;
        RECURRING_BALANCE.save(deps.storage, &user, &balance)?;
    }

    // Update current executing request id
    state.curr_executing_request_id = id;
    STATE.save(deps.storage, &state)?;

    // Forward escrowed assets and execute contract
    let mut msgs = vec![];

    if let Some(input_asset) = request.input_asset.clone() {
        let target = deps.api.addr_validate(&request.target)?;
        msgs.push(SubMsg {
            id: 0,
            msg: input_asset.into_msg(&deps.querier, target)?,
            gas_limit: None,
            reply_on: ReplyOn::Never,
        });
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

    // Transfer fee to executor
    let fee_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.fee_denom,
        },
        amount: config.fee_amount,
    };
    msgs.push(SubMsg {
        id: 0,
        msg: fee_asset.into_msg(&deps.querier, info.sender.clone())?,
        gas_limit: None,
        reply_on: ReplyOn::Never,
    });

    // Remove request
    if !request.is_recurring {
        let mut state = STATE.load(deps.storage)?;
        state.total_requests -= 1;
        STATE.save(deps.storage, &state)?;
        REQUESTS.remove(deps.storage, id);
    }

    Ok(Response::new().add_submessages(msgs).add_attributes(vec![
        attr("action", "execute_request"),
        attr("id", id.to_string()),
        attr("executor", info.sender),
    ]))
}

/// ## Description
/// Deposit recurring fee. Returns a [`ContractError`] on failure.
/// * Fails [`recurring_count`] is invalid.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **recurring_count** is count of recurring, defines the deposit amount
/// * it is an object of type [`u64`].
pub fn deposit_recurring_fee(
    deps: DepsMut,
    info: MessageInfo,
    recurring_count: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let deposit_amount = config
        .fee_amount
        .checked_mul(Uint128::from(recurring_count))?;

    // Check if `recurring_count` is zero
    if recurring_count == 0 || deposit_amount != must_pay(&info, &config.fee_denom)? {
        return Err(ContractError::InvalidRecurringCount {});
    }

    // Update storage
    let mut state = STATE.load(deps.storage)?;
    let mut balance = RECURRING_BALANCE
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    state.total_recurring_fee += deposit_amount;
    balance += deposit_amount;

    STATE.save(deps.storage, &state)?;
    RECURRING_BALANCE.save(deps.storage, &info.sender, &balance)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "deposit_recurring_fee"),
        attr("recurring_count", recurring_count.to_string()),
        attr("amount", deposit_amount.to_string()),
    ]))
}

/// ## Description
/// Withdraw recurring fee. Returns a [`ContractError`] on failure.
/// * Fails [`recurring_count`] is invalid.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **recurring_count** is count of recurring, defines the deposit amount
/// * it is an object of type [`u64`].
pub fn withdraw_recurring_fee(
    deps: DepsMut,
    info: MessageInfo,
    recurring_count: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let withdraw_amount = config
        .fee_amount
        .checked_mul(Uint128::from(recurring_count))?;

    let mut state = STATE.load(deps.storage)?;
    let mut balance = RECURRING_BALANCE
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    // Validate withdraw amount
    if balance < withdraw_amount {
        return Err(ContractError::InvalidRecurringCount {});
    }

    // Update state
    balance -= withdraw_amount;
    state.total_recurring_fee -= withdraw_amount;
    STATE.save(deps.storage, &state)?;
    RECURRING_BALANCE.save(deps.storage, &info.sender, &balance)?;

    // Transfer asset
    let withdraw_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.fee_denom,
        },
        amount: withdraw_amount,
    };
    Ok(Response::new()
        .add_message(withdraw_asset.into_msg(&deps.querier, info.sender)?)
        .add_attributes(vec![
            attr("action", "withdraw_recurring_fee"),
            attr("recurring_count", recurring_count.to_string()),
            attr("amount", withdraw_amount.to_string()),
        ]))
}

/// ## Description
/// Process when we receive AUTO tokens for staking. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **cw20_msg** is an object of type [`Cw20ReceiveMsg`].
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

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

/// ## Description
/// Process stakings when AUTO is native token. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **num_stakes** is the number of stakings, which is an object of type [`u64`].
pub fn receive_denom(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    num_stakes: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match config.auto {
        AssetInfo::Token { contract_addr: _ } => Err(CommonError::Unauthorized {}.into()),
        AssetInfo::NativeToken { denom } => {
            let received_auto = must_pay(&info, &denom)?;
            let staker = info.clone().sender;
            stake(deps, env, info, &staker, num_stakes, received_auto)
        }
    }
}

/// ## Description
/// Update stakes for new stakings. Returns a [`ContractError`] on failure.
/// * Add user address to array `num_stakes` times
/// * Update user's and total staking balances
/// * Update executor
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **_info** is an object of type [`MessageInfo`].
///
/// * **sender** is the staker, which is an object of type [`Addr`].
///
/// * **num_stakes** is the number of stakings, which is an object of type [`u64`].
///
/// * **amount** is the staking amount, which is an object of type [`Uint128`].
pub fn stake(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: &Addr,
    num_stakes: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Validate amount
    if config.stake_amount * Uint128::from(num_stakes) != amount {
        return Err(ContractError::InvalidStakeInfo {});
    }

    // Update executor
    let mut state = STATE.load(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    STATE.save(deps.storage, &state)?;

    // Update stakes array
    for _ in 0..num_stakes {
        state.stakes.push(sender.to_string());
    }

    // Add amount to stake balance
    let balance = STAKE_BALANCE.load(deps.storage, sender).unwrap_or_default() + amount;
    STAKE_BALANCE.save(deps.storage, sender, &balance)?;
    state.total_staked += amount;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "stake"),
        attr("user", sender),
        attr("num_stakes", num_stakes.to_string()),
    ]))
}

/// ## Description
/// Unstake AUTO. Returns a [`ContractError`] on failure.
/// * Remove from stakes array at indexes of [`idxs`]
/// * Return staked AUTO
/// * Updates executor
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **idxs** is the index array of stakings, which is an object of type [`Vec<u64>`].
pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    idxs: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Update executor
    let mut state = STATE.load(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    STATE.save(deps.storage, &state)?;

    // Validate and remove stakes
    for idx in &idxs {
        let idx = *idx as usize;
        if idx >= state.stakes.len() {
            return Err(ContractError::IdxOutOfBound {});
        }
        let addr = deps.api.addr_validate(&state.stakes[idx])?;
        if addr != info.sender {
            return Err(ContractError::IdxNotYou {});
        }
        state.stakes.swap_remove(idx);
    }

    // Update stake balance
    let amount = Uint128::from(idxs.len() as u64) * config.stake_amount;
    let balance = STAKE_BALANCE
        .load(deps.storage, &info.sender)
        .unwrap_or_default()
        - amount;
    STAKE_BALANCE.save(deps.storage, &info.sender, &balance)?;
    state.total_staked -= amount;
    STATE.save(deps.storage, &state)?;

    // Return assets
    let return_asset = Asset {
        info: config.auto,
        amount,
    };

    Ok(Response::new()
        .add_message(return_asset.into_msg(&deps.querier, info.sender.clone())?)
        .add_attributes(vec![
            attr("action", "unstake"),
            attr("user", info.sender),
            attr("count", idxs.len().to_string()),
        ]))
}

/// ## Description
/// Util fcn for executor update
/// * It first checks the executor is set for current epoch
/// * If not, decide current epoch and set the executor
/// * If nobody staked yet, then executor is set to empty string
///
/// ## Params
/// * **state** is current [`STATE`]
///
/// * **env** is an object of type [`Env`].
///
/// * **blocks_in_epoch** is block counts included in a single epoch.
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

/// ## Description
/// Update executor for current epoch. Returns a [`ContractError`] on failure.
/// * It first checks the executor is set for current epoch
/// * If not, decide current epoch and set the executor
/// * If nobody staked yet, then executor is set to empty string
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **env** is an object of type [`Env`].
pub fn update_executor(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut state = STATE.load(deps.storage)?;
    _update_executor(&mut state, env, config.blocks_in_epoch);
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_executor"),
        attr("epoch", state.last_epoch.to_string()),
        attr("executor", state.executor),
    ]))
}

/// ## Description
/// Add new addresses to blacklist. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **addrs** is a list of addresses, which is an object of type [`Vec<String>`].
///
/// ## Executor
/// Only the admin can execute this.
pub fn add_to_blacklist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addrs: Vec<String>,
) -> Result<Response, ContractError> {
    // Only admin can update blacklist
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    for addr_str in addrs {
        let addr = deps.api.addr_validate(&addr_str)?;
        BLACKLIST.save(deps.storage, &addr, &addr_str)?;
    }

    Ok(Response::new().add_attribute("action", "add_to_blacklist"))
}

/// ## Description
/// Remove addresses to blacklist. Returns a [`ContractError`] on failure.
///
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **info** is an object of type [`MessageInfo`].
///
/// * **addrs** is a list of addresses, which is an object of type [`Vec<String>`].
///
/// ## Executor
/// Only the admin can execute this.
pub fn remove_from_blacklist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addrs: Vec<String>,
) -> Result<Response, ContractError> {
    // Only admin can update blacklist
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    for addr_str in addrs {
        let addr = deps.api.addr_validate(&addr_str)?;
        BLACKLIST.remove(deps.storage, &addr);
    }

    Ok(Response::new().add_attribute("action", "remove_from_blacklist"))
}

/// ## Description
/// The entry point to the contract for processing replies from submessages.
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **msg** is an object of type [`Reply`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => execute_reply(deps, env, msg.result),
        _ => Err(CommonError::Unauthorized {}.into()),
    }
}

/// ## Description
/// Sets the `curr_executing_request_id` back to default value
/// ## Params
/// * **deps** is an object of type [`DepsMut`].
///
/// * **_env** is an object of type [`Env`].
///
/// * **_msg** is an object of type [`SubMsgResult`].
pub fn execute_reply(
    deps: DepsMut,
    _env: Env,
    _msg: SubMsgResult,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    state.curr_executing_request_id = u64::MAX;
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attributes(vec![attr("action", "finalize_execute")]))
}

/// ## Description
/// Exposes all the queries available in the contract.
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **env** is an object of type [`Env`].
///
/// * **msg** is an object of type [`QueryMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),

        QueryMsg::PendingAdmin {} => to_binary(&NEW_ADMIN.query_admin(deps)?),

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

        QueryMsg::RecurringFees { user } => Ok(to_binary(&query_recurring_fees(deps, user)?)?),

        QueryMsg::StakeAmount { user } => Ok(to_binary(&query_stake_amount(deps, user)?)?),

        QueryMsg::Stakes { start, limit } => Ok(to_binary(&query_stakes(deps, start, limit)?)?),

        QueryMsg::Blacklist {} => Ok(to_binary(&query_blacklist(deps)?)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`Config`] structure.
///
/// ## Params
/// * **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// ## Description
/// Return info of reqeust with `id` using [`RequestInfoResponse`] structure.
///
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **id** is the request id.
pub fn query_request_info(deps: Deps, id: u64) -> StdResult<RequestInfoResponse> {
    let info = REQUESTS.load(deps.storage, id).unwrap_or(Request {
        user: zero_string(),
        target: zero_string(),
        msg: to_binary("")?,
        input_asset: None,
        is_recurring: false,
        created_at: 0,
    });
    Ok(RequestInfoResponse { id, request: info })
}

/// ## Description
/// Returns an array with request data that contains items of type [`Request`]. Querying starts at `start_after` and returns `limit` pairs.
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **start_after** is an [`Option`] field which accepts an id of type [`u64`].
/// This is the request from which we start to query.
///
/// * **limit** is a [`Option`] type. Sets the number of requests to be retrieved.
///
/// * **order_by** is a [`OrderBy`] type.
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

/// ## Description
/// Return current state of requests and stakes using [`StateResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    let resp = StateResponse {
        curr_executing_request_id: state.curr_executing_request_id,
        total_requests: state.total_requests,
        total_recurring_fee: state.total_recurring_fee,
        next_request_id: state.next_request_id,
        total_stake_amount: state.total_staked,
        stakes_len: state.stakes.len() as u64,
    };

    Ok(resp)
}

/// ## Description
/// Return current state of requests and stakes using [`EpochInfoResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **env** is an object of type [`Env`].
pub fn query_epoch_info(deps: Deps, env: Env) -> StdResult<EpochInfoResponse> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let cur_epoch = env.block.height / config.blocks_in_epoch * config.blocks_in_epoch;
    let resp = EpochInfoResponse {
        cur_epoch,
        last_epoch: state.last_epoch,
        executor: state.executor,
    };

    Ok(resp)
}

/// ## Description
/// Return recurring fee amount of the user using [`RecurringFeeAmountResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **user** is a [`String`] which is the address of the user.
pub fn query_recurring_fees(deps: Deps, user: String) -> StdResult<RecurringFeeAmountResponse> {
    let amount = RECURRING_BALANCE
        .load(deps.storage, &deps.api.addr_validate(&user)?)
        .unwrap_or_default();
    let resp = RecurringFeeAmountResponse { amount };

    Ok(resp)
}

/// ## Description
/// Return staked amount of the user using [`StakeAmountResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **user** is a [`String`] which is the address of the user.
pub fn query_stake_amount(deps: Deps, user: String) -> StdResult<StakeAmountResponse> {
    let amount = STAKE_BALANCE
        .load(deps.storage, &deps.api.addr_validate(&user)?)
        .unwrap_or_default();
    let resp = StakeAmountResponse { amount };

    Ok(resp)
}

/// ## Description
/// Return stakings from `start` with limit of `limit` as [`StakesResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
///
/// * **start** is starting id of the stakings.
///
/// * **limit** is a [`u64`] of return size limit.
pub fn query_stakes(deps: Deps, start: u64, limit: u64) -> StdResult<StakesResponse> {
    let state = STATE.load(deps.storage)?;

    let mut end = (start + limit) as usize;
    if end > state.stakes.len() {
        end = state.stakes.len()
    };
    let start = start as usize;

    Ok(StakesResponse {
        stakes: state.stakes[start..end].to_vec(),
    })
}

/// ## Description
/// Return blacklist as [`BlacklistResponse`]
/// ## Params
/// * **deps** is an object of type [`Deps`].
pub fn query_blacklist(deps: Deps) -> StdResult<BlacklistResponse> {
    let addrs: StdResult<Vec<(Addr, String)>> = BLACKLIST
        .range(deps.storage, None, None, OrderBy::Asc.into())
        .collect();

    let blacklist: StdResult<Vec<String>> =
        addrs?.iter().map(|record| Ok(record.1.clone())).collect();

    Ok(BlacklistResponse {
        blacklist: blacklist?,
    })
}
