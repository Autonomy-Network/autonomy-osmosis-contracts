// -------------------------------------------------------------------------------------
// LocalOsmosis test-suite
// -------------------------------------------------------------------------------------
import chalk from "chalk";
import { GasPrice } from "@cosmjs/stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";

import { localosmosis } from "./config/localosmosisConstants";

import { setupCommon } from "./setup";

import { testExecuteWrapperOsmosis } from "./tests/wrapper";
import { testExecuteRegistry } from "./tests/registry";

// -------------------------------------------------------------------------------------
// Variables
// -------------------------------------------------------------------------------------

let client: SigningCosmWasmClient;
let wallet1: DirectSecp256k1HdWallet;
let wallet2: DirectSecp256k1HdWallet; // reg_user_fee_veri_forwarder
let wallet3: DirectSecp256k1HdWallet; // router_user_fee_veri_forwarder

// Autonomy common contracts
let auto: string;
let registry: string;
let testToken: string;

// WrapperOsmosis test
let outDenom: string;
let inDenom: string;
let poolId: number;
let wrapperOsmosis: string;

// -------------------------------------------------------------------------------------
// initialize autonomy-station variables
// -------------------------------------------------------------------------------------
async function initializeCommon() {
  wallet1 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet1,
    { prefix: "osmo" }
  );
  wallet2 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet2,
    { prefix: "osmo" }
  );
  wallet3 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet3,
    { prefix: "osmo" }
  );

  client = await SigningCosmWasmClient.connectWithSigner(
    localosmosis.networkInfo.url,
    wallet1,
    { gasPrice: GasPrice.fromString("0.1uosmo") }
  );

  const [account1] = await wallet1.getAccounts();
  const [account2] = await wallet2.getAccounts();
  const [account3] = await wallet3.getAccounts();

  console.log(`Use ${chalk.cyan(account1.address)} as Wallet 1`);
  console.log(`Use ${chalk.cyan(account2.address)} as Wallet 2`);
  console.log(`Use ${chalk.cyan(account3.address)} as Wallet 3`);

  auto = localosmosis.contracts.auto;
  registry = localosmosis.contracts.registry;
  testToken = localosmosis.contracts.testToken;

  console.log(`Use ${chalk.cyan(auto)} as AUTO token`);
  console.log(`Use ${chalk.cyan(registry)} as Registry`);
}

// -------------------------------------------------------------------------------------
// initialize WrapperOsmosis variables
// -------------------------------------------------------------------------------------
async function initializeWrapperOsmosis() {
  wallet1 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet1,
    { prefix: "osmo" }
  );
  wallet2 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet2,
    { prefix: "osmo" }
  );
  wallet3 = await DirectSecp256k1HdWallet.fromMnemonic(
    localosmosis.mnemonicKeys.wallet3,
    { prefix: "osmo" }
  );

  client = await SigningCosmWasmClient.connectWithSigner(
    localosmosis.networkInfo.url,
    wallet1,
    { gasPrice: GasPrice.fromString("0.1uosmo") }
  );

  const [account1] = await wallet1.getAccounts();
  const [account2] = await wallet2.getAccounts();
  const [account3] = await wallet3.getAccounts();

  console.log(`Use ${chalk.cyan(account1.address)} as Wallet 1`);
  console.log(`Use ${chalk.cyan(account2.address)} as Wallet 2`);
  console.log(`Use ${chalk.cyan(account3.address)} as Wallet 3`);

  auto = localosmosis.contracts.auto;
  registry = localosmosis.contracts.registry;
  wrapperOsmosis = localosmosis.contracts.wrapperOsmosis;
  poolId = localosmosis.poolInfo.poolId;
  outDenom = localosmosis.poolInfo.outDenom;
  inDenom = localosmosis.poolInfo.inDenom;

  console.log(`Use ${chalk.cyan(auto)} as AUTO token`);
  console.log(`Use ${chalk.cyan(registry)} as Registry`);
  console.log(`Use ${chalk.cyan(wrapperOsmosis)} as WrapperOsmosis`);
  console.log(`Use ${chalk.cyan(poolId)} as PoolId`);
  console.log(`Use ${chalk.cyan(outDenom)} as Out Denom`);
}

// -------------------------------------------------------------------------------------
// setup autonomy common contracts
// -------------------------------------------------------------------------------------
export async function startSetupCommon(): Promise<void> {
  console.log(chalk.blue("\nTestNet"));

  // Initialize environment information
  console.log(chalk.yellow("\nStep 1. Environment Info"));
  await initializeCommon();

  // Setup contracts
  console.log(chalk.yellow("\nStep 2. Common Contracts Setup"));
  await setupCommon(client, { wallet1, wallet2, wallet3 });
}

// -------------------------------------------------------------------------------------
// Wrapper
// -------------------------------------------------------------------------------------
export async function startTestWrapper(): Promise<void> {
  console.log(chalk.blue("\nTestNet"));

  // Initialize environment information
  console.log(chalk.yellow("\nStep 1. Environment Info"));
  await initializeWrapperOsmosis();

  // Test queries
  await testExecuteWrapperOsmosis(
    client,
    wallet1,
    wallet2,
    wallet3,
    auto,
    registry,
    poolId,
    inDenom,
    outDenom,
    wrapperOsmosis
  );
}

// -------------------------------------------------------------------------------------
// Registry
// -------------------------------------------------------------------------------------
export async function startTestRegistry(): Promise<void> {
  console.log(chalk.blue("\nTestNet"));

  // Initialize environment information
  console.log(chalk.yellow("\nStep 1. Environment Info"));
  await initializeCommon();

  // Test registry
  await testExecuteRegistry(client, testToken, auto, registry);
}
