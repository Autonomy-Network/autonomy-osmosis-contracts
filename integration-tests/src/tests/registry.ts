import chalk from "chalk";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Coin, coin, DirectSecp256k1HdWallet, Registry } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";

import { localosmosis } from "../config/localosmosisConstants";
import {toEncodedBinary } from "../helpers";
import { WrapperClient } from "../ts/wrapper/Wrapper.client";
import { RegistryClient } from "../ts/registry/Registry.client";

chai.use(chaiAsPromised);
const { expect } = chai;

export async function testExecuteRegistry(
  client: SigningCosmWasmClient,
  auto: string,
  registry: string,
): Promise<void> {
  console.log(chalk.yellow("\nStep 2. Running Tests"));

  await testStaking(client, auto, registry);
  await testManageRequests(client, registry);

  process.exit();
}

// -----------------------------------------------
//  TEST: "Registry" successfully manages stakes and excutors
//
//  SCENARIO:
//    Stake AUTO
//    Check AUTO amount and contract info.
//    Unstake AUTO.
//    Check AUTO amount and contract info.
// ------------------------------------------------
async function testStaking(
  client: SigningCosmWasmClient,
  auto: string,
  registry: string,
): Promise<void> {
  process.stdout.write("Test - Staking");

  const registryClient1 = new RegistryClient(client, localosmosis.addresses.wallet1, registry);

  const config = await registryClient1.config();
  const state0 = await registryClient1.state();

  // Stake asset
  const numStakes = 12;
  const stakeAmount = parseInt(config.stake_amount) * numStakes;
  const accBal0 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal0 = await client.getBalance(registry, auto);
  const txnRes0 = await registryClient1.stakeDenom({
    numStakes,
  }, "auto", undefined, [coin(stakeAmount, auto)]);
  const accBal1 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal1 = await client.getBalance(registry, auto);

  // expect(parseInt(accBal0.amount) - stakeAmount - txnRes0.gasUsed).to.be.equal(parseInt(accBal1.amount));
  expect(parseInt(conBal0.amount) + stakeAmount).to.be.equal(parseInt(conBal1.amount));

  const state1 = await registryClient1.state();
  expect(state0.stakes_len + numStakes).to.be.equal(state1.stakes_len);
  expect(parseInt(state0.total_stake_amount) + stakeAmount).to.be.equal(parseInt(state1.total_stake_amount));

  // Unstake asset
  const unstakeCount = 3;
  const unstakeAmount = parseInt(config.stake_amount) * unstakeCount;
  const txnRes1 = await registryClient1.unstake({ idxs: [ state0.stakes_len, state0.stakes_len, state0.stakes_len ] })
  const accBal2 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal2 = await client.getBalance(registry, auto);

  // expect(parseInt(accBal1.amount) + unstakeAmount - txnRes1.gasUsed).to.be.equal(parseInt(accBal2.amount));
  expect(parseInt(conBal1.amount) - unstakeAmount).to.be.equal(parseInt(conBal2.amount));

  const state2 = await registryClient1.state();
  expect(state1.stakes_len - unstakeCount).to.be.equal(state2.stakes_len);
  expect(parseInt(state1.total_stake_amount) - unstakeAmount).to.be.equal(parseInt(state2.total_stake_amount));

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: "Registry" successfully creates and cancel requests
//
//  SCENARIO:
//    Create new request.
//    Check request is correctly updated inside queue.
//    Cancel reqeust.
//    Check request is correctly removed from the queue.
//    Check fee asset is correctly returned
// ------------------------------------------------
async function testManageRequests(
  client: SigningCosmWasmClient,
  registry: string,
): Promise<void> {
  process.stdout.write("Test - Request management by registry");

  const registryClient1 = new RegistryClient(client, localosmosis.addresses.wallet1, registry);

  const config = await registryClient1.config();
  const state0 = await registryClient1.state();

  // Create Request
  await registryClient1.updateExecutor();

  const fee = coin(config.fee_amount, config.fee_denom);
  const requestInfo = {
    target: registry,
    msg: toEncodedBinary({
      cancel_request: {
        id: 0,
      }
    }),
    input_asset: {
      info: {
        native_token: {
          denom: "uosmo"
        }
      },
      amount: "0"
    }
  };
  await registryClient1.createRequest({
    requestInfo,
  }, "auto", undefined, [fee]);

  // Check request
  const state1 = await registryClient1.state();
  const request1 = await registryClient1.requestInfo({ id: state0.next_request_id });
  expect(state0.total_requests + 1).to.be.equal(state1.total_requests);
  expect(request1.request.target).to.be.equal(requestInfo.target);
  expect(request1.request.msg).to.be.equal(requestInfo.msg);
  // expect(request1.request.input_asset).to.be.equal(requestInfo.input_asset);

  // Cancel request
  const feeBalance0 = await client.getBalance(localosmosis.addresses.wallet1, config.fee_denom);
  const txInfo = await registryClient1.cancelRequest({
    id: state0.next_request_id
  });

  // Check request
  const state2 = await registryClient1.state();
  const request2 = await registryClient1.requestInfo({ id: state0.next_request_id });
  const feeBalance1 = await client.getBalance(localosmosis.addresses.wallet1, config.fee_denom);
  expect(state0.total_requests).to.be.equal(state2.total_requests);
  expect(request2.request.target).to.be.equal("");
  // expect(parseInt(feeBalance0.amount) + parseInt(config.fee_amount) - txInfo.gasUsed).to.be.equal(parseInt(feeBalance1.amount));

  console.log(chalk.green(" Passed!"));
}
