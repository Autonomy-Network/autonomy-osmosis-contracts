/* eslint-disable @typescript-eslint/no-explicit-any */
import chalk from "chalk";
import { storeCode, instantiateContract } from "./helpers";
import { wasm_path } from "./config/wasmPaths";

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";

import { localosmosis } from "./config/localosmosisConstants";
import { GasPrice } from "@cosmjs/stargate";

import { CreateOrUpdateConfig } from "./ts/registry/Registry.types";
import { RegistryClient } from "./ts/registry/Registry.client";

// -------------------------------------------------------------------------------------
// Variables
// -------------------------------------------------------------------------------------

let client: SigningCosmWasmClient;
let wallet1: DirectSecp256k1HdWallet;
let wallet2: DirectSecp256k1HdWallet; // reg_user_fee_veri_forwarder
let wallet3: DirectSecp256k1HdWallet; // router_user_fee_veri_forwarder

let auto: string;
let registry: string;
let wrapperOsmosis: string;
let cw20: string;

// -------------------------------------------------------------------------------------
// setup all contracts for LocalOsmosis
// -------------------------------------------------------------------------------------
export async function setupCommon(
  client: SigningCosmWasmClient,
  wallets: {
    wallet1: DirectSecp256k1HdWallet;
    wallet2: DirectSecp256k1HdWallet;
    wallet3: DirectSecp256k1HdWallet;
  }
): Promise<void> {
  client = client;
  wallet1 = wallets.wallet1;
  wallet2 = wallets.wallet2;
  wallet3 = wallets.wallet3;

  // Send some tokens to wallets
  await client.sendTokens(
    localosmosis.addresses.wallet1,
    localosmosis.addresses.wallet2,
    [coin("100000000", "uion"), coin("100000000", "uosmo")],
    "auto"
  );
  await client.sendTokens(
    localosmosis.addresses.wallet1,
    localosmosis.addresses.wallet3,
    [coin("100000000", "uion"), coin("100000000", "uosmo")],
    "auto"
  );

  await setup(client, wallet1);

  // Stake some AUTOs to become a executor
  const registryClient = new RegistryClient(
    client,
    localosmosis.addresses.wallet1,
    registry
  );
  await registryClient.stakeDenom({ numStakes: 1 }, "auto", undefined, [
    coin("10000", auto),
  ]);

  console.log(chalk.green(" Done!"));
  process.exit();
}

async function setup(
  client: SigningCosmWasmClient,
  wallet1: DirectSecp256k1HdWallet
): Promise<void> {
  // Step 1. Upload all local wasm files and capture the codes for each....

  process.stdout.write("Uploading CW20 Token Wasm");
  const cw20CodeId = await storeCode(
    client,
    wallet1,
    `${wasm_path.station}/cw20_base.wasm`
  );
  console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${cw20CodeId}`);

  process.stdout.write("Uploading Registry Wasm");
  const registryCodeId = await storeCode(
    client,
    wallet1,
    `${wasm_path.station}/registry_stake.wasm`
  );
  console.log(
    chalk.green(" Done!"),
    `${chalk.blue("codeId")} = ${registryCodeId}`
  );

  process.stdout.write("Uploading wrapperOsmosis Wasm");
  const wrapperOsmosisCodeId = await storeCode(
    client,
    wallet1,
    `${wasm_path.station}/wrapper_osmosis.wasm`
  );
  console.log(
    chalk.green(" Done!"),
    `${chalk.blue("codeId")} = ${wrapperOsmosisCodeId}`
  );

  // Step 2. Instantiate contracts
  auto = "uosmo";

  process.stdout.write("Instantiating Cw20 contract");
  const cw20Result = await instantiateContract(
    client,
    wallet1,
    wallet1,
    cw20CodeId,
    {
      name: "Test TOKEN",
      symbol: "TEST",
      decimals: 6,
      initial_balances: [
        {
          address: localosmosis.addresses.wallet1,
          amount: "10000000000",
        },
        {
          address: localosmosis.addresses.wallet2,
          amount: "10000000000",
        },
        {
          address: localosmosis.addresses.wallet3,
          amount: "10000000000",
        },
      ],
    }
  );
  cw20 = cw20Result.contractAddress;
  console.log(
    chalk.green(" Done!"),
    `${chalk.blue("contractAddress")}=${cw20}`
  );

  // registry
  process.stdout.write("Instantiating Registry contract");

  const registryConfig: CreateOrUpdateConfig = {
    auto: {
      native_token: {
        denom: auto,
      },
    },
    blocks_in_epoch: 100,
    fee_amount: "1000",
    fee_denom: "uosmo",
    owner: localosmosis.addresses.wallet1,
    stake_amount: "10000",
  };
  const registryResult = await instantiateContract(
    client,
    wallet1,
    wallet1,
    registryCodeId,
    {
      config: registryConfig,
    }
  );
  registry = registryResult.contractAddress;
  console.log(
    chalk.green(" Done!"),
    `${chalk.blue("contractAddress")}=${registry}`
  );

  process.stdout.write("Instantiating Wrapper contract");
  const wrapperOsmosisResult = await instantiateContract(
    client,
    wallet1,
    wallet1,
    wrapperOsmosisCodeId,
    {}
  );
  wrapperOsmosis = wrapperOsmosisResult.contractAddress;
  console.log(
    chalk.green(" Done!"),
    `${chalk.blue("contractAddress")}=${wrapperOsmosis}`
  );
}
