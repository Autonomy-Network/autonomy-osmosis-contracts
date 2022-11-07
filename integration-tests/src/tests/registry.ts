import chalk from "chalk";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import {
  Coin,
  coin
} from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";

import { localosmosis } from "../config/localosmosisConstants";
import { toEncodedBinary } from "../helpers";
import { WrapperClient } from "../ts/wrapper/Wrapper.client";
import { RegistryClient } from "../ts/registry/Registry.client";
import { assert } from "console";

chai.use(chaiAsPromised);
const { expect } = chai;

export async function testExecuteRegistry(
  client: SigningCosmWasmClient,
  testToken: string,
  auto: string,
  registry: string
): Promise<void> {
  console.log(chalk.yellow("\nStep 2. Running Tests"));

  await testStaking(client, auto, registry);
  await testManageRequests(client, registry);
  await testExecuteRequests(client, testToken, registry);
  await testRecurringRequests(client, testToken, registry);

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
  registry: string
): Promise<void> {
  process.stdout.write("Test - Staking");

  const registryClient1 = new RegistryClient(
    client,
    localosmosis.addresses.wallet1,
    registry
  );

  const config = await registryClient1.config();
  const state0 = await registryClient1.state();

  // Stake asset
  const numStakes = 12;
  const stakeAmount = parseInt(config.stake_amount) * numStakes;
  const accBal0 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal0 = await client.getBalance(registry, auto);
  const txnRes0 = await registryClient1.stakeDenom(
    {
      numStakes,
    },
    "auto",
    undefined,
    [coin(stakeAmount, auto)]
  );
  const accBal1 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal1 = await client.getBalance(registry, auto);

  // expect(parseInt(accBal0.amount) - stakeAmount - txnRes0.gasUsed).to.be.equal(parseInt(accBal1.amount));
  expect(parseInt(conBal0.amount) + stakeAmount).to.be.equal(
    parseInt(conBal1.amount)
  );

  const state1 = await registryClient1.state();
  expect(state0.stakes_len + numStakes).to.be.equal(state1.stakes_len);
  expect(parseInt(state0.total_stake_amount) + stakeAmount).to.be.equal(
    parseInt(state1.total_stake_amount)
  );

  // Unstake asset
  const unstakeCount = 3;
  const unstakeAmount = parseInt(config.stake_amount) * unstakeCount;
  const txnRes1 = await registryClient1.unstake({
    idxs: [state0.stakes_len, state0.stakes_len, state0.stakes_len],
  });
  const accBal2 = await client.getBalance(localosmosis.addresses.wallet1, auto);
  const conBal2 = await client.getBalance(registry, auto);

  // expect(parseInt(accBal1.amount) + unstakeAmount - txnRes1.gasUsed).to.be.equal(parseInt(accBal2.amount));
  expect(parseInt(conBal1.amount) - unstakeAmount).to.be.equal(
    parseInt(conBal2.amount)
  );

  const state2 = await registryClient1.state();
  expect(state1.stakes_len - unstakeCount).to.be.equal(state2.stakes_len);
  expect(parseInt(state1.total_stake_amount) - unstakeAmount).to.be.equal(
    parseInt(state2.total_stake_amount)
  );

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
  registry: string
): Promise<void> {
  process.stdout.write("Test - Request management by registry");

  const registryClient1 = new RegistryClient(
    client,
    localosmosis.addresses.wallet1,
    registry
  );

  const config = await registryClient1.config();
  const state0 = await registryClient1.state();

  // Create Request
  const fee = coin(config.fee_amount, config.fee_denom);
  const requestInfo = {
    target: registry,
    msg: toEncodedBinary({
      cancel_request: {
        id: 0,
      },
    }),
    is_recurring: false,
  };
  await registryClient1.createRequest(
    {
      requestInfo,
    },
    "auto",
    undefined,
    [fee]
  );

  // Check request
  const state1 = await registryClient1.state();
  const request1 = await registryClient1.requestInfo({
    id: state0.next_request_id,
  });
  expect(state0.total_requests + 1).to.be.equal(state1.total_requests);
  expect(request1.request.target).to.be.equal(requestInfo.target);
  expect(request1.request.msg).to.be.equal(requestInfo.msg);
  // expect(request1.request.input_asset).to.be.equal(requestInfo.input_asset);

  // Cancel request
  const feeBalance0 = await client.getBalance(
    localosmosis.addresses.wallet1,
    config.fee_denom
  );
  const txInfo = await registryClient1.cancelRequest({
    id: state0.next_request_id,
  });

  // Check request
  const state2 = await registryClient1.state();
  const request2 = await registryClient1.requestInfo({
    id: state0.next_request_id,
  });
  const feeBalance1 = await client.getBalance(
    localosmosis.addresses.wallet1,
    config.fee_denom
  );
  expect(state0.total_requests).to.be.equal(state2.total_requests);
  expect(request2.request.target).to.be.equal("");
  // expect(parseInt(feeBalance0.amount) + parseInt(config.fee_amount) - txInfo.gasUsed).to.be.equal(parseInt(feeBalance1.amount));

  console.log(chalk.green(" Passed!"));
}

async function testExecuteRequests(
  client: SigningCosmWasmClient,
  testToken: string,
  registry: string
): Promise<void> {
  process.stdout.write("Test - Execute Requests");

  const registryClient = new RegistryClient(
    client,
    localosmosis.addresses.wallet1,
    registry
  );
  const config = await registryClient.config();
  const transferAmount = "10000000";

  const state0 = await registryClient.state();
  const totalRequests0 = state0.total_requests;

  // 1. Create request
  let transferMsg = toEncodedBinary({
    transfer: {
      recipient: localosmosis.addresses.wallet2,
      amount: transferAmount,
    },
  });
  await registryClient.createRequest(
    {
      requestInfo: {
        target: testToken,
        msg: transferMsg,
        is_recurring: false,
      },
    },
    "auto",
    undefined,
    [coin(config.fee_amount, config.fee_denom)]
  );

  // 2. Check if the request created
  const requestId = state0.next_request_id;
  const request = await registryClient.requestInfo({
    id: requestId,
  });
  expect(request.request.target).to.be.equal(testToken);
  expect(request.request.msg).to.be.equal(transferMsg);
  expect(request.request.is_recurring).to.be.equal(false);

  // 3. Executes the request
  const beforeTcw = await client.queryContractSmart(testToken, {
    balance: { address: localosmosis.addresses.wallet2 },
  });
  const beforeTcwBalance = beforeTcw.balance;

  const beforeUosmo: Coin = await client.getBalance(
    localosmosis.addresses.wallet1,
    "uosmo"
  );
  const beforeUosmoBalance = beforeUosmo.amount;

  await client.execute(
    localosmosis.addresses.wallet1,
    testToken,
    {
      transfer: {
        recipient: registry,
        amount: transferAmount,
      },
    },
    "auto"
  );
  await registryClient.updateExecutor();
  await registryClient.executeRequest(
    {
      id: requestId,
    },
    "auto"
  );

  // Wallet2 receives some tokens.
  const afterTcw = await client.queryContractSmart(testToken, {
    balance: { address: localosmosis.addresses.wallet2 },
  });
  const afterTcwBalance = afterTcw.balance;

  const afterUosmo: Coin = await client.getBalance(
    localosmosis.addresses.wallet1,
    "uosmo"
  );
  const afterUosmoBalance = afterUosmo.amount;

  expect(parseInt(afterTcwBalance)).to.be.equal(parseInt(beforeTcwBalance) + parseInt(transferAmount));
  expect(parseInt(afterUosmoBalance)).to.be.not.equal(parseInt(beforeUosmoBalance));

  const totalRequests1 = (await registryClient.state()).total_requests;
  const requests: any = await registryClient.requests({});
  expect(requests.requests.length)
    .to.be.equal(totalRequests0)
    .to.be.equal(totalRequests1);

  console.log(chalk.green(" Passed!"));
}

async function testRecurringRequests(
  client: SigningCosmWasmClient,
  testToken: string,
  registry: string
): Promise<void> {
  process.stdout.write("Test - Recurring Requests");

  const registryClient = new RegistryClient(
    client,
    localosmosis.addresses.wallet1,
    registry
  );
  const config = await registryClient.config();

  // Deposit into fee pool
  const recurringCount = 10;
  const feeAmount = parseInt(config.fee_amount) * recurringCount;
  const state0 = await registryClient.state();
  const recurInfo0 = await registryClient.recurringFees({
    user: localosmosis.addresses.wallet1,
  });
  await registryClient.depositRecurringFee(
    { recurringCount },
    "auto",
    undefined,
    [coin(feeAmount, config.fee_denom)]
  );
  const recurInfo1 = await registryClient.recurringFees({
    user: localosmosis.addresses.wallet1,
  });
  const state1 = await registryClient.state();
  expect(parseInt(state0.total_recurring_fee) + feeAmount).to.be.equal(
    parseInt(state1.total_recurring_fee)
  );
  expect(parseInt(recurInfo0.amount) + feeAmount).to.be.equal(
    parseInt(recurInfo1.amount)
  );

  // Withdraw from fee pool
  const withdrawCount = 8;
  const withdrawAmount = parseInt(config.fee_amount) * withdrawCount;
  const balance0 = await client.getBalance(registry, config.fee_denom);
  await registryClient.withdrawRecurringFee(
    { recurringCount: withdrawCount },
    "auto"
  );
  const balance1 = await client.getBalance(registry, config.fee_denom);
  const state2 = await registryClient.state();
  const recurInfo2 = await registryClient.recurringFees({
    user: localosmosis.addresses.wallet1,
  });
  expect(parseInt(state2.total_recurring_fee) + withdrawAmount).to.be.equal(
    parseInt(state1.total_recurring_fee)
  );
  expect(parseInt(balance1.amount) + withdrawAmount).to.be.equal(
    parseInt(balance0.amount)
  );
  expect(parseInt(recurInfo2.amount) + withdrawAmount).to.be.equal(
    parseInt(recurInfo1.amount)
  );

  // Create request
  const totalRequests0 = state2.total_requests;
  const transferAmount = "100000";
  let transferMsg = toEncodedBinary({
    transfer: {
      recipient: localosmosis.addresses.wallet2,
      amount: transferAmount,
    },
  });
  // Recurring request can't have input_asset
  await expect(registryClient.createRequest(
    {
      requestInfo: {
        target: testToken,
        msg: transferMsg,
        is_recurring: true,
        input_asset: {
          info: {
            native_token: {
              denom: "uosmo"
            }
          },
          amount: "10"
        }
      },
    },
    "auto"
  )).to.be.rejected;
  await registryClient.createRequest(
    {
      requestInfo: {
        target: testToken,
        msg: transferMsg,
        is_recurring: true,
      },
    },
    "auto"
  );

  // Executes the request
  const requestId = state2.next_request_id;
  await client.execute(
    localosmosis.addresses.wallet1,
    testToken,
    {
      transfer: {
        recipient: registry,
        amount: transferAmount,
      },
    },
    "auto"
  );
  await registryClient.updateExecutor();
  await registryClient.executeRequest(
    {
      id: requestId,
    },
    "auto"
  );

  const recurInfo3 = await registryClient.recurringFees({
    user: localosmosis.addresses.wallet1,
  });
  const state3 = await registryClient.state();
  const balance2 = await client.getBalance(registry, config.fee_denom);

  expect(
    parseInt(state3.total_recurring_fee) + parseInt(config.fee_amount)
  ).to.be.equal(parseInt(state2.total_recurring_fee));
  expect(parseInt(balance2.amount) + parseInt(config.fee_amount)).to.be.equal(
    parseInt(balance1.amount)
  );
  expect(parseInt(recurInfo3.amount) + parseInt(config.fee_amount)).to.be.equal(
    parseInt(recurInfo2.amount)
  );

  const totalRequests1 = (await registryClient.state()).total_requests;

  // Try again
  await client.execute(
    localosmosis.addresses.wallet1,
    testToken,
    {
      transfer: {
        recipient: registry,
        amount: transferAmount,
      },
    },
    "auto"
  );
  await registryClient.updateExecutor();
  await registryClient.executeRequest(
    {
      id: requestId,
    },
    "auto"
  );
  const totalRequests2 = (await registryClient.state()).total_requests;
  const requests: any = await registryClient.requests({});
  expect(requests.requests.length)
    .to.be.equal(totalRequests0 + 1)
    .to.be.equal(totalRequests1)
    .to.be.equal(totalRequests2);
  // Rejects cuz no fee remaining
  await client.execute(
    localosmosis.addresses.wallet1,
    testToken,
    {
      transfer: {
        recipient: registry,
        amount: transferAmount,
      },
    },
    "auto"
  );
  await registryClient.updateExecutor();
  await expect(
    registryClient.executeRequest(
      {
        id: requestId,
      },
      "auto"
    )
  ).to.be.rejected;

  console.log(chalk.green(" Passed!"));
}
