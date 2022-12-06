use autonomy::{
    asset::{Asset, AssetInfo},
    types::OrderBy,
};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Binary, StdResult, Storage, Uint128};

use cw_controllers::Admin;

/// Protocol configuration
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    /// Amount of request execution fee
    pub fee_amount: Uint128,

    /// Asset denom of request execution fee; we will limit to OSMO for osmosis
    pub fee_denom: String,

    /// AUTO token for executors
    pub auto: AssetInfo,

    /// Single stake amount
    pub stake_amount: Uint128,

    /// Blocks in a single epoch
    pub blocks_in_epoch: u64,
}

/// Current state of the registry and stakes
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct State {
    /// Registry

    /// Request Id of currently being executed
    pub curr_executing_request_id: u64,
    /// Id of the request will be created for next
    pub next_request_id: u64,
    /// Number of total requests in the queue
    pub total_requests: u64,
    /// Total recurring fee amount
    pub total_recurring_fee: Uint128,

    /// Staking

    /// Total amount of staked AUTO
    pub total_staked: Uint128,
    /// Address list of all stakers
    pub stakes: Vec<String>,
    /// Last epoch for executor rotation
    pub last_epoch: u64,
    /// Address of executor in the last epoch
    pub executor: String,
}

/// Actual request struct
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Request {
    /// The user who registered this request.
    pub user: String,

    /// Target contract.
    pub target: String,

    /// Msg to call the target
    pub msg: Binary,

    /// Asset sent in advance
    pub input_asset: Option<Asset>,

    /// Recurring request
    pub is_recurring: bool,

    /// Timestamp for creation
    pub created_at: u64,
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub const ADMIN: Admin = Admin::new("admin");
pub const NEW_ADMIN: Admin = Admin::new("new_admin");

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");

pub const STAKE_BALANCE: Map<&Addr, Uint128> = Map::new("stake_balance");
pub const RECURRING_BALANCE: Map<&Addr, Uint128> = Map::new("recurring_balance");
pub const REQUESTS: Map<u64, Request> = Map::new("requests");

pub fn read_requests(
    storage: &dyn Storage,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<(u64, Request)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    REQUESTS
        .range(
            storage,
            start,
            None,
            order_by.unwrap_or(OrderBy::Asc).into(),
        )
        .take(limit)
        .collect()
}
