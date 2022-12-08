use autonomy::{
    asset::{Asset, AssetInfo},
    types::OrderBy,
};
use cosmwasm_std::{Binary, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Request;

/// Config struct to initialze or update configuration
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct CreateOrUpdateConfig {
    /// Contract admin
    pub admin: Option<String>,

    /// Amount of request execution fee
    pub fee_amount: Option<Uint128>,

    /// Asset denom of request execution fee
    pub fee_denom: Option<String>,

    /// AUTO token for executors
    pub auto: Option<AssetInfo>,

    /// Single stake amount
    pub stake_amount: Option<Uint128>,

    /// Blocks in a single epoch
    pub blocks_in_epoch: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub config: CreateOrUpdateConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct CreateRequestInfo {
    /// Target contract to call for this request
    pub target: String,

    /// Msg for the target contract
    pub msg: Binary,

    /// Assets used for this call
    pub input_asset: Option<Asset>,

    /// Is this recurring request?
    pub is_recurring: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Claim Admin
    ClaimAdmin { },
    /// Update Config
    UpdateConfig { config: CreateOrUpdateConfig },

    /// Registry

    /// Create a new execution request
    CreateRequest { request_info: CreateRequestInfo },
    /// Cancel a request with `id`
    CancelRequest { id: u64 },
    /// Execute a request with `id`
    ExecuteRequest { id: u64 },
    /// Deposit into recurring fee pool
    DepositRecurringFee { recurring_count: u64 },
    /// Withdraw from recurring fee pool
    WithdrawRecurringFee { recurring_count: u64 },

    /// Staking

    /// Implemention for cw20 receive msg, when staking
    Receive(Cw20ReceiveMsg),
    /// Staking when execution fee is native asset
    /// `num_stakes` is the number of staking
    StakeDenom { num_stakes: u64 },
    /// Unstake stakers of the caller at index array of `idxs`
    Unstake { idxs: Vec<u64> },
    /// Update executor for current epoch
    UpdateExecutor {},

    /// Black list

    /// Add to blacklist
    AddToBlacklist { addrs: Vec<String> },
    /// Remove from blacklist
    RemoveFromBlacklist { addrs: Vec<String> }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Stake AUTO to be an executor
    Stake { num_stakes: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get registry admin
    Admin {},
    /// Get registry config
    Config {},
    /// Get current state of registry
    State {},
    /// Get recurring info of ther user
    RecurringFees { user: String },
    /// Get details of a single request
    RequestInfo { id: u64 },
    /// Get many requests
    Requests {
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    },
    /// Get current executor rotation epoch info
    EpochInfo {},
    /// Get staked amount of a user
    StakeAmount { user: String },
    /// Get array of staked addresses
    Stakes { start: u64, limit: u64 },
    /// Get array of blacklisted addresses
    Blacklist { },
}

/// Response for query registry state
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StateResponse {
    // Request id of the request being executed
    pub curr_executing_request_id: u64,

    // Count of total queued requests
    pub total_requests: u64,

    /// Total recurring fee amount
    pub total_recurring_fee: Uint128,

    /// Id of the request will be created for next
    pub next_request_id: u64,

    /// Total amount of staked AUTO
    pub total_stake_amount: Uint128,

    /// Lenght of stakes array
    pub stakes_len: u64,
}

/// Response for single request query
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct RequestInfoResponse {
    pub id: u64,
    pub request: Request,
}

/// Response for query many requests
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct RequestsResponse {
    pub requests: Vec<RequestInfoResponse>,
}

/// Response for current epoch info
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct EpochInfoResponse {
    /// Current Epoch from current block timestamp
    pub cur_epoch: u64,

    /// Epoch of last excutor update
    pub last_epoch: u64,

    /// Last updated executor
    pub executor: String,
}

/// Response for staked amount of a user
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct RecurringFeeAmountResponse {
    pub amount: Uint128,
}

/// Response for staked amount of a user
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StakeAmountResponse {
    pub amount: Uint128,
}

/// Response for staked list
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StakesResponse {
    pub stakes: Vec<String>,
}

/// Response for staked list
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct BlacklistResponse {
    pub blacklist: Vec<String>,
}
