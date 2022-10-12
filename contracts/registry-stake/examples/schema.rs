use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use registry_stake::{
    msg::{
        CreateOrUpdateConfig, CreateRequestInfo, EpochInfoResponse, ExecuteMsg, InstantiateMsg,
        QueryMsg, RequestInfoResponse, RequestsResponse, StakeAmountResponse, StakesResponse,
        StateResponse,
    },
    state::{Config, State},
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(RequestInfoResponse), &out_dir);
    export_schema(&schema_for!(RequestsResponse), &out_dir);
    export_schema(&schema_for!(StateResponse), &out_dir);
    export_schema(&schema_for!(StakeAmountResponse), &out_dir);
    export_schema(&schema_for!(StakesResponse), &out_dir);
    export_schema(&schema_for!(EpochInfoResponse), &out_dir);
    export_schema(&schema_for!(CreateOrUpdateConfig), &out_dir);
    export_schema(&schema_for!(CreateRequestInfo), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
}
