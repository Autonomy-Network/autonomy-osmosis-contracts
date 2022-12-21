use std::vec;

use crate::contract::{execute, instantiate, query, reply, migrate};
use crate::error::ContractError;
use crate::msg::{
    BlacklistResponse, CreateOrUpdateConfig, CreateRequestInfo, Cw20HookMsg, EpochInfoResponse,
    ExecuteMsg, InstantiateMsg, QueryMsg, RecurringFeeAmountResponse, RequestInfoResponse,
    RequestsResponse, StakeAmountResponse, StakesResponse, StateResponse, MigrateMsg,
};
use crate::state::{Config, Request};
use crate::testing::mock_querier::mock_dependencies;

use autonomy::asset::{Asset, AssetInfo};
use autonomy::error::CommonError;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Coin, CosmosMsg, SubMsg, Timestamp, Uint128, WasmMsg, ReplyOn, BankMsg, Reply, SubMsgResult, SubMsgResponse, Response,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_controllers::{AdminError, AdminResponse};

#[test]
fn proper_initialization_migrate() {
    let mut deps = mock_dependencies(&[]);

    let admin = Addr::unchecked("admin");

    let mut config = CreateOrUpdateConfig {
        admin: None,
        fee_amount: Some(Uint128::from(10000u128)),
        fee_denom: Some("utest".to_string()),
        auto: Some(AssetInfo::Token {
            contract_addr: Addr::unchecked("auto"),
        }),
        stake_amount: Some(Uint128::from(1000u128)),
        blocks_in_epoch: Some(1),
    };
    assert_eq!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("addr0000", &[]),
            InstantiateMsg {
                config: config.clone(),
            }
        )
        .err(),
        Some(CommonError::InstantiateParamsUnavailable {}.into())
    );

    config.admin = Some(admin.to_string());
    let _res = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("addr0000", &[]),
        InstantiateMsg {
            config: config.clone(),
        },
    );

    assert_eq!(
        from_binary::<AdminResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::Admin {}).unwrap()
        )
        .unwrap(),
        AdminResponse {
            admin: Some(admin.to_string())
        }
    );

    assert_eq!(
        from_binary::<Config>(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
            .unwrap(),
        Config {
            fee_amount: config.fee_amount.unwrap(),
            fee_denom: config.fee_denom.unwrap(),
            auto: config.auto.unwrap(),
            stake_amount: config.stake_amount.unwrap(),
            blocks_in_epoch: config.blocks_in_epoch.unwrap()
        }
    );

    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 0,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );

    // migrate
    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg{}).unwrap();
    assert_eq!(res, Response::default());
}

#[test]
fn test_claim_admin() {
    let mut deps = mock_dependencies(&[]);

    // Instantiate
    let admin = Addr::unchecked("admin");
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some(admin.to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg);

    // Update admin
    let new_admin = Addr::unchecked("new_admin");
    let msg = ExecuteMsg::UpdateConfig {
        config: CreateOrUpdateConfig {
            admin: Some(new_admin.to_string()),
            fee_amount: None,
            fee_denom: None,
            auto: None,
            stake_amount: None,
            blocks_in_epoch: None,
        },
    };
    let info = mock_info("admin", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config"),]);
    assert_eq!(
        from_binary::<AdminResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::PendingAdmin {}).unwrap()
        )
        .unwrap(),
        AdminResponse {
            admin: Some(new_admin.to_string())
        }
    );

    // Claim admin error
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("addr0000", &[]),
        ExecuteMsg::ClaimAdmin {},
    );
    assert_eq!(res.err(), Some(AdminError::NotAdmin {}.into()));

    // Claim admin
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_admin.as_ref(), &[]),
        ExecuteMsg::ClaimAdmin {},
    )
    .unwrap();
    assert_eq!(
        from_binary::<AdminResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::Admin {}).unwrap()
        )
        .unwrap(),
        AdminResponse {
            admin: Some(new_admin.to_string())
        }
    );
    assert_eq!(
        from_binary::<AdminResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::PendingAdmin {}).unwrap()
        )
        .unwrap(),
        AdminResponse { admin: None }
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_admin"),
            attr("new admin", new_admin.to_string())
        ]
    );
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies(&[]);

    // Instantiate
    let admin = Addr::unchecked("admin");
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some(admin.to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

    // Update config
    let new_admin = Addr::unchecked("new_admin");
    let msg = ExecuteMsg::UpdateConfig {
        config: CreateOrUpdateConfig {
            admin: Some(new_admin.to_string()),
            fee_amount: None,
            fee_denom: None,
            auto: None,
            stake_amount: Some(Uint128::from(2000u128)),
            blocks_in_epoch: None,
        },
    };
    // With wrong admin
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(AdminError::NotAdmin {}.into()));

    // With right admin but wrong config
    let info = mock_info("admin", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).err();
    assert_eq!(err, Some(ContractError::UpdateConfigError {}));

    // Update with right admin
    let new_config = CreateOrUpdateConfig {
        admin: Some(new_admin.to_string()),
        fee_amount: Some(Uint128::from(100200u128)),
        fee_denom: Some("utestt".to_string()),
        auto: None,
        stake_amount: None,
        blocks_in_epoch: Some(12),
    };
    let msg = ExecuteMsg::UpdateConfig {
        config: new_config.clone(),
    };
    let info = mock_info("admin", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config"),]);
    let config =
        from_binary::<Config>(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
            .unwrap();
    assert_eq!(
        config,
        Config {
            fee_amount: new_config.fee_amount.unwrap(),
            fee_denom: new_config.fee_denom.unwrap(),
            auto: config.auto.clone(),
            stake_amount: config.stake_amount,
            blocks_in_epoch: new_config.blocks_in_epoch.unwrap()
        }
    );
}

#[test]
fn test_blacklist() {
    let mut deps = mock_dependencies(&[]);

    // Instantiate
    let admin = Addr::unchecked("admin");
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some(admin.to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

    let msg = ExecuteMsg::AddToBlacklist {
        addrs: vec!["contract000".to_string(), "contract001".to_string()],
    };
    // With wrong admin
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(AdminError::NotAdmin {}.into()));

    // With right admin
    let info = mock_info("admin", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "add_to_blacklist".to_string())]
    );

    // Check blacklist
    let blacklist = from_binary::<BlacklistResponse>(
        &query(deps.as_ref(), mock_env(), QueryMsg::Blacklist {}).unwrap(),
    )
    .unwrap();
    assert_eq!(
        blacklist.blacklist,
        vec!["contract000".to_string(), "contract001".to_string()]
    );

    // Create request fails
    let request_info = CreateRequestInfo {
        target: "contract001".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: None,
        is_recurring: true,
    };
    let info = mock_info("addr0000", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::CreateRequest {
            request_info,
        },
    )
    .err();
    assert_eq!(err, Some(ContractError::TargetBlacklisted {}));

    // Remove from blacklist
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::RemoveFromBlacklist {
        addrs: vec!["contract000".to_string()],
    };
    // With wrong admin
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(AdminError::NotAdmin {}.into()));

    // With right admin
    let info = mock_info("admin", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "remove_from_blacklist".to_string())]
    );

    // Check blacklist
    let blacklist = from_binary::<BlacklistResponse>(
        &query(deps.as_ref(), mock_env(), QueryMsg::Blacklist {}).unwrap(),
    )
    .unwrap();
    assert_eq!(blacklist.blacklist, vec!["contract001".to_string()]);
}

#[test]
fn test_create_request() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        &[
            Coin::new(10000000u128, "uosmo".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);

    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // NoInputAssetForRecurring
    let input_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: "uosmo".to_string(),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset.clone()),
        is_recurring: true,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };

    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(10000u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(10u128),
            },
        ],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg).err();
    assert_eq!(err, Some(ContractError::NoInputAssetForRecurring {}));

    // Insufficient Fee
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info: request_info.clone(),
    };

    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(9999u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(10u128),
            },
        ],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(ContractError::InsufficientFee {}));

    // NoFeePaid
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(ContractError::NoFeePaid {}));

    // InvalidInputAssets
    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(10000u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(9u128),
            },
        ],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(ContractError::InvalidInputAssets {}));

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).err();
    assert_eq!(err, Some(ContractError::InvalidInputAssets {}));

    // Successful creation
    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(10000u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(10u128),
            },
        ],
    );
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1000);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 1,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 1,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );
    assert_eq!(
        from_binary::<RequestInfoResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::RequestInfo { id: 0 }).unwrap()
        )
        .unwrap(),
        RequestInfoResponse {
            id: 0,
            request: Request {
                user: "addr0000".to_string(),
                target: request_info.target,
                msg: request_info.msg,
                input_asset: request_info.input_asset,
                is_recurring: request_info.is_recurring,
                created_at: env.block.time.seconds()
            }
        }
    );

    // Create with token, non-native
    let input_asset = Asset {
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("contract000".to_string()),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset.clone()),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "contract000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: "addr0000".to_string(),
                recipient: MOCK_CONTRACT_ADDR.to_string(),
                amount: input_asset.amount,
            })
            .unwrap(),
            funds: vec![],
        }))],
    );

    // no input asset, recurring
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: None,
        is_recurring: true,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info: request_info.clone(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1000);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_request"),
            attr("id", "2".to_string()),
            attr("user", "addr0000".to_string()),
            attr("target", request_info.target.clone()),
            attr("msg", request_info.msg.to_string()),
            attr("asset", "None"),
            attr("is_recurring", "true"),
            attr("created_at", "1000".to_string()),
        ],
    );

    // Ensure state
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 3,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 3,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );

    let requests = from_binary::<RequestsResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Requests {
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        requests.requests[2],
        RequestInfoResponse {
            id: 2,
            request: Request {
                user: "addr0000".to_string(),
                target: request_info.target,
                msg: request_info.msg,
                input_asset: request_info.input_asset,
                is_recurring: request_info.is_recurring,
                created_at: env.block.time.seconds()
            }
        }
    );

    let requests = from_binary::<RequestsResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Requests {
                start_after: Some(0),
                limit: None,
                order_by: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(requests.requests.len(), 2);
}

#[test]
fn test_cancel_request() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        &[
            Coin::new(10000000u128, "uosmo".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);

    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 0
    let input_asset0 = Asset {
        info: AssetInfo::NativeToken {
            denom: "uosmo".to_string(),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset0),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };
    let info = mock_info(
        "addr0000",

        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(10000u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(10u128),
            },
        ],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 1, Create with token, non-native
    let input_asset = Asset {
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("contract000".to_string()),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 2, no input asset, recurring
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: None,
        is_recurring: true,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info
    };
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Unauthorized
    let info = mock_info("another_user", &[]);
    let msg = ExecuteMsg::CancelRequest { id: 0 };
    let err = execute(deps.as_mut(), mock_env(), info, msg).err();
    assert_eq!(err, Some(CommonError::Unauthorized {}.into()));

    // Successful cancel
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::CancelRequest { id: 0 };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "cancel_request"),
            attr("id", "0".to_string()),
        ],
    );

    // Ensure state
    assert_eq!(
        from_binary::<RequestInfoResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::RequestInfo { id: 0 }).unwrap()
        )
        .unwrap(),
        RequestInfoResponse {
            id: 0,
            request: Request {
                user: "".to_string(),
                target: "".to_string(),
                msg: to_binary("").unwrap(),
                input_asset: None,
                is_recurring: false,
                created_at: 0
            }
        }
    );

    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 2,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 3,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );

    let requests = from_binary::<RequestsResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Requests {
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(requests.requests.len(), 2);
}

#[test]
fn test_stake_denom() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::NativeToken {
                denom: "uauto".to_string(),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // InvalidStakeInfo
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::StakeDenom { num_stakes: 1 },
    )
    .err();
    assert_eq!(err, Some(ContractError::InvalidStakeInfo {}));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "stake"),
            attr("user", "addr0000".to_string()),
            attr("num_stakes", "2".to_string()),
        ]
    );

    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 0,
            total_stake_amount: Uint128::from(4000u128),
            stakes_len: 4
        }
    );
    assert_eq!(
        from_binary::<StakeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StakeAmount {
                    user: "addr0000".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        StakeAmountResponse {
            amount: Uint128::from(4000u128)
        }
    );
    let stakes = from_binary::<StakesResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Stakes { start: 0, limit: 6 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stakes.stakes,
        vec![
            "addr0000".to_string(),
            "addr0000".to_string(),
            "addr0000".to_string(),
            "addr0000".to_string(),
        ]
    );

    // error when stake cw20 called
    let info = mock_info("auto", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "addr0000".to_string(),
            amount: Uint128::from(2000u128),
            msg: to_binary(&Cw20HookMsg::Stake { num_stakes: 2 }).unwrap(),
        }),
    )
    .err();
    assert_eq!(err, Some(ContractError::InvalidAutoToken {}));
}

#[test]
fn test_stake_cw20() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::Token {
                contract_addr: Addr::unchecked("auto"),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // InvalidStakeInfo
    let info = mock_info("contract0000", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "addr0000".to_string(),
            amount: Uint128::from(2000u128),
            msg: to_binary("").unwrap(),
        }),
    )
    .err();
    assert_eq!(err, Some(ContractError::DataShouldBeGiven {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "addr0000".to_string(),
            amount: Uint128::from(2000u128),
            msg: to_binary(&Cw20HookMsg::Stake { num_stakes: 2 }).unwrap(),
        }),
    )
    .err();
    assert_eq!(err, Some(CommonError::Unauthorized {}.into()));

    let info = mock_info("auto", &[]);
    let res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "addr0000".to_string(),
            amount: Uint128::from(2000u128),
            msg: to_binary(&Cw20HookMsg::Stake { num_stakes: 2 }).unwrap(),
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "stake"),
            attr("user", "addr0000".to_string()),
            attr("num_stakes", "2".to_string()),
        ]
    );

    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 0,
            total_stake_amount: Uint128::from(2000u128),
            stakes_len: 2
        }
    );
    assert_eq!(
        from_binary::<StakeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StakeAmount {
                    user: "addr0000".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        StakeAmountResponse {
            amount: Uint128::from(2000u128)
        }
    );
    let stakes = from_binary::<StakesResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Stakes { start: 0, limit: 2 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stakes.stakes,
        vec!["addr0000".to_string(), "addr0000".to_string(),]
    );

    // error when stake denom
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 1 },
    )
    .err();
    assert_eq!(err, Some(CommonError::Unauthorized {}.into()));
}

#[test]
fn test_unstake() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    deps.querier.with_balance(&[(
        &"addr1".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::NativeToken {
                denom: "uauto".to_string(),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Stake `addr0` 2 times
    let info = mock_info(
        "addr0",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();

    // Stake `addr1` once
    let info = mock_info(
        "addr1",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 1 },
    )
    .unwrap();

    // Stake `addr0` once
    let info = mock_info(
        "addr0",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 1 },
    )
    .unwrap();

    // Stake `addr1` twice
    let info = mock_info(
        "addr1",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();

    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 0,
            total_stake_amount: Uint128::from(6000u128),
            stakes_len: 6
        }
    );
    assert_eq!(
        from_binary::<StakeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StakeAmount {
                    user: "addr0".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        StakeAmountResponse {
            amount: Uint128::from(3000u128)
        }
    );
    let stakes = from_binary::<StakesResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Stakes { start: 0, limit: 6 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stakes.stakes,
        vec![
            "addr0".to_string(),
            "addr0".to_string(),
            "addr1".to_string(),
            "addr0".to_string(),
            "addr1".to_string(),
            "addr1".to_string(),
        ]
    );

    // IdxOutOfBound
    let info = mock_info("addr0", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unstake { idxs: vec![7] },
    )
    .err();
    assert_eq!(err, Some(ContractError::IdxOutOfBound {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Unstake { idxs: vec![2] },
    )
    .err();
    assert_eq!(err, Some(ContractError::IdxNotYou {}));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Unstake {
            idxs: vec![0, 1, 3],
        },
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unstake"),
            attr("user", "addr0".to_string()),
            attr("count", "3".to_string()),
        ]
    );

    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 0,
            total_stake_amount: Uint128::from(3000u128),
            stakes_len: 3
        }
    );
    assert_eq!(
        from_binary::<StakeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StakeAmount {
                    user: "addr0".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        StakeAmountResponse {
            amount: Uint128::zero()
        }
    );
    let stakes = from_binary::<StakesResponse>(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Stakes { start: 0, limit: 3 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stakes.stakes,
        vec![
            "addr1".to_string(),
            "addr1".to_string(),
            "addr1".to_string(),
        ]
    );
}

#[test]
fn test_update_executor() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::NativeToken {
                denom: "uauto".to_string(),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(100),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Stake `addr0` 2 times
    let info = mock_info(
        "addr0",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();

    // Update executor
    let mut env = mock_env();
    env.block.height = 100001;
    let last_epoch = env.block.height / 100 * 100;
    let info = mock_info("addr0", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::UpdateExecutor {},
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_executor"),
            attr("epoch", last_epoch.to_string()),
            attr("executor", "addr0".to_string()),
        ]
    );

    env.block.height = 1002201;
    let cur_epoch = env.block.height / 100 * 100;
    assert_eq!(
        from_binary::<EpochInfoResponse>(
            &query(deps.as_ref(), env, QueryMsg::EpochInfo {}).unwrap()
        )
        .unwrap(),
        EpochInfoResponse {
            cur_epoch,
            last_epoch,
            executor: "addr0".to_string()
        }
    );
}

#[test]
fn test_recurring_fee() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::NativeToken {
                denom: "uauto".to_string(),
            }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(1),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // InvalidRecurringCount
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(20000u128),
        }],
    );
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DepositRecurringFee { recurring_count: 1 },
    )
    .err();
    assert_eq!(err, Some(ContractError::InvalidRecurringCount {}));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DepositRecurringFee { recurring_count: 2 },
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_recurring_fee"),
            attr("recurring_count", "2".to_string()),
            attr("amount", "20000".to_string()),
        ]
    );

    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::DepositRecurringFee { recurring_count: 2 },
    )
    .unwrap();
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::from(40000u128),
            next_request_id: 0,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );
    assert_eq!(
        from_binary::<RecurringFeeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RecurringFees {
                    user: "addr0000".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        RecurringFeeAmountResponse {
            amount: Uint128::from(40000u128)
        }
    );

    // withdraw
    let info = mock_info("addr0000", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::WithdrawRecurringFee { recurring_count: 8 },
    )
    .err();
    assert_eq!(err, Some(ContractError::InvalidRecurringCount {}));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::WithdrawRecurringFee { recurring_count: 3 },
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw_recurring_fee"),
            attr("recurring_count", "3".to_string()),
            attr("amount", "30000".to_string()),
        ]
    );
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 0,
            total_recurring_fee: Uint128::from(10000u128),
            next_request_id: 0,
            total_stake_amount: Uint128::zero(),
            stakes_len: 0
        }
    );
    assert_eq!(
        from_binary::<RecurringFeeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RecurringFees {
                    user: "addr0000".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        RecurringFeeAmountResponse {
            amount: Uint128::from(10000u128)
        }
    );
}

#[test]
fn test_execute_request() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        &[
            Coin::new(10000000u128, "uosmo".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    deps.querier.with_balance(&[(
        &"executor".to_string(),
        &[
            Coin::new(10000000u128, "uauto".to_string()),
            Coin::new(10000000u128, "utest".to_string()),
        ],
    )]);
    let msg = InstantiateMsg {
        config: CreateOrUpdateConfig {
            admin: Some("admin".to_string()),
            fee_amount: Some(Uint128::from(10000u128)),
            fee_denom: Some("utest".to_string()),
            auto: Some(AssetInfo::NativeToken { denom: "uauto".to_string() }),
            stake_amount: Some(Uint128::from(1000u128)),
            blocks_in_epoch: Some(100),
        },
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 0
    let input_asset0 = Asset {
        info: AssetInfo::NativeToken {
            denom: "uosmo".to_string(),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset0),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };
    let info = mock_info(
        "addr0000",

        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(10000u128),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::from(10u128),
            },
        ],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 1, Create with token, non-native
    let input_asset = Asset {
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("contract000".to_string()),
        },
        amount: Uint128::from(10u128),
    };
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: Some(input_asset),
        is_recurring: false,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Request 2, no input asset, recurring
    let request_info = CreateRequestInfo {
        target: "contract0000".to_string(),
        msg: to_binary("").unwrap(),
        input_asset: None,
        is_recurring: true,
    };
    let msg = ExecuteMsg::CreateRequest {
        request_info,
    };
    let info = mock_info(
        "addr0000",

        &[Coin {
            denom: "utest".to_string(),
            amount: Uint128::from(10000u128),
        }],
    );
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Executors need to stake
    let info = mock_info(
        "executor",
        &[Coin {
            denom: "uauto".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::StakeDenom { num_stakes: 2 },
    )
    .unwrap();

    // Without executor updated
    let info = mock_info("executor", &[]);
    let mut env = mock_env();
    env.block.height = 100001;
    let epoch = env.block.height / 100 * 100;
    let _res = execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecuteRequest { id: 1 },
    )
    .unwrap();
    assert_eq!(
        from_binary::<EpochInfoResponse>(
            &query(deps.as_ref(), env.clone(), QueryMsg::EpochInfo {}).unwrap()
        )
        .unwrap(),
        EpochInfoResponse {
            cur_epoch: epoch,
            last_epoch: epoch,
            executor: "executor".to_string()
        }
    );

    // Update Executor
    env.block.height = 100002;
    let info = mock_info("addr0", &[]);
    let _res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateExecutor {},
    )
    .unwrap();

    // TODO: InvalidExecutor
    let info = mock_info("not_executor", &[]);
    let mut env = mock_env();
    env.block.height = 100003;
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ExecuteRequest { id: 0 },
    )
    .err();
    assert_eq!(
        err,
        Some(ContractError::InvalidExecutor {})
    );

    // Execution success
    let info = mock_info("executor", &[]);
    let mut env = mock_env();
    env.block.height = 100003;
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ExecuteRequest { id: 0 },
    )
    .unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg {
                id: 0,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "contract0000".to_string(),
                    amount: vec![Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::from(10u128)
                    }],
                }),
                gas_limit: None,
                reply_on: ReplyOn::Never,
            },
            SubMsg {
                id: 1,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "contract0000".to_string(),
                    msg: to_binary("").unwrap(),
                    funds: vec![],
                }),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            },
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "executor".to_string(),
                amount: vec![Coin {
                    denom: "utest".to_string(),
                    amount: Uint128::from(10000u128)
                }],
            }))
        ],
    );
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: 0,
            total_requests: 1,
            total_recurring_fee: Uint128::zero(),
            next_request_id: 3,
            total_stake_amount: Uint128::from(2000u128),
            stakes_len: 2
        }
    );

    // Insufficient Recurring Fee
    let info = mock_info("executor", &[]);
    let mut env = mock_env();
    env.block.height = 100003;
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ExecuteRequest { id: 2 },
    )
    .err();
    assert_eq!(
        err,
        Some(ContractError::InsufficientRecurringFee {})
    );

    // Deposit recurring fee
    let info = mock_info(
        "addr0000",

        &[
            Coin {
                denom: "utest".to_string(),
                amount: Uint128::from(20000u128),
            },
        ],
    );
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::DepositRecurringFee { recurring_count: 2 },
    )
    .unwrap();

    // Execution success
    let info = mock_info("executor", &[]);
    let mut env = mock_env();
    env.block.height = 100003;
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ExecuteRequest { id: 2 },
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "execute_request"),
            attr("id", "2".to_string()),
            attr("executor", "executor".to_string()),
        ]
    );

    // Check results
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: 2,
            total_requests: 1,
            total_recurring_fee: Uint128::from(10000u128),
            next_request_id: 3,
            total_stake_amount: Uint128::from(2000u128),
            stakes_len: 2
        }
    );
    assert_eq!(
        from_binary::<RequestInfoResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::RequestInfo { id: 0 }).unwrap()
        )
        .unwrap(),
        RequestInfoResponse {
            id: 0,
            request: Request {
                user: "".to_string(),
                target: "".to_string(),
                msg: to_binary("").unwrap(),
                input_asset: None,
                is_recurring: false,
                created_at: 0
            }
        }
    );
    // recurring request still exists
    assert_eq!(
        from_binary::<RequestInfoResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::RequestInfo { id: 2 }).unwrap()
        )
        .unwrap().id,
        2
    );
    assert_eq!(
        from_binary::<RecurringFeeAmountResponse>(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RecurringFees {
                    user: "addr0000".to_string()
                }
            )
            .unwrap()
        )
        .unwrap(),
        RecurringFeeAmountResponse {
            amount: Uint128::from(10000u128)
        }
    );

    // Reply
    let err = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 0,
            result: SubMsgResult::Ok(SubMsgResponse { events: vec![], data: None }),
        }
    )
    .err();
    assert_eq!(
        err,
        Some(CommonError::Unauthorized { }.into())
    );
    let res = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse { events: vec![], data: None }),
        }
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "finalize_execute")]
    );
    assert_eq!(
        from_binary::<StateResponse>(
            &query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap()
        )
        .unwrap(),
        StateResponse {
            curr_executing_request_id: u64::MAX,
            total_requests: 1,
            total_recurring_fee: Uint128::from(10000u128),
            next_request_id: 3,
            total_stake_amount: Uint128::from(2000u128),
            stakes_len: 2
        }
    );
}
