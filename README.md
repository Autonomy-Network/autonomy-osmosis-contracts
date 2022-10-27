## Autonomy Network for Osmosis

Autonomy Network is a decentralized automation protocol - it allows you to have transactions automatically execute in the future under arbitrary conditions.
It's an infrastructure service, and a tool, that allows anyone to automate any on-chain action with any on-chain condition. For example, limit orders/stop losses for traders of decentralized exchanges like Uniswap, and exit liquidity positions once impermanent loss becomes too great. A blog post giving a simple overview of Autonomy and each component can be found here. Autonomy is full composable, so it can even be used by on-chain entities like DAOs to automate things such as recurring salary payments. Because it's infrastructure that's meant to be used by other projects directly to add features, similar to ChainLink, users don't even need to know it exists - they just use a dapp that automates something, such as a limit order, and the experience is the same as it would be with a centralized exchange. Users don't need to learn anything or deploy a contract, it 'just works'!
Autonomy is ultimately a B2B tool. However, the team has been creating user-facing dapps to prove the feasibility and demand for new use cases, using them as a stepping stone for the automation features being integrated into other projects natively or having others fork our dapps and improve them while still using Autonomy under the hood.

[Documentation for Autonomy Network](https://autonomy-network.gitbook.io/autonomy-docs/autonomy-network/overview)

### Contracts for Osmosis

Osmosis contracts consist of Registry and Wrapper.

#### Registry-Stake contract

Registry stores requests. Users can stake their AUTO tokens into Registry in order to execute requests for their epoch. Things are generally automated by using the Autonomy Registry in conjunction with a wrapper contract around some system - for example like a wrapper around the Osmosis DEX that allows for limits and stops on Osmosis. A 2nd simultaneous proposal will deploy this wrapper contract to add limits and stops to Osmosis.

##### Registry Functions

- Create new requests
Creators needs to specify `target contract`, `msg` and `assets` that will be spent for the call.
When creating a new request, users should escrow `assets for the execution` and `execution fee`.
- Cancel a request
Creator can cancel a request he/she has created. By canceling, he/she gets the escrowed `execution assets` and `execution fee` back.
Canceled request is removed from the storage.
- Execute a request
Executor of a request is set at its creation.
Only executors can execute the request.
By executing requests, they earn execution fees.
Executed request is removed from the storage.
- How executor is set
A number of blocks at a certain period is called an epoch.
Each epoch has its executor randomly chosen from the stakers.
When a new request is being created, its `executor` is set to the `executor of the epoch` at creating moment.

##### What is recurring request?

Recurring request is not removed from the queue after execution in order to recur the request execution.
Users should deposit their execution fees into the recurring fee pool. The balance in this pool is reduced every time the request is executed.
So, when creating a recurring request, the user doesn't need to pay the execution fee.
As for now, recurring requests don't have input assets for request execution.

##### Staking Functions

- Stake AUTO token in order to be an executor.
The contract stores an array of staker address list.
Each element represent a chance of 1/N to be chosen as an executor for an epoch. (N = array length)
To be added to an array once, user needs to stake `STAKE_AMOUNT`.
So, if A has staked `STAKE_AMOUNT` * 3 amount  of AUTO, the array becomes [...] -> [..., A, A, A].
- Unstake AUTO
Unstaking is done by passing the indexes of the array.
User can point only indexes of his own.
e.g. if the array is [A, B, B, A, A, B, A], then A's indexes are [0, 3, 4, 6].
But removal is done from the first which means the A's indexes will be updated while removal.
So in the above case, if A wanted to remove all of his stakes, then he needs to pass [0, 2, 2, 3].
- Update executor
If the epoch info stored inside contract is old, then we should update it.
Executor for the epoch is chosen randomly from the stakes array.
`stakes[rand % stakes_len]`
We use `oorandom` for random number generation.

#### Wrapper-Osmosis contract

Wrapper contract executes the swap operation between assets and validate results, when used with the Autonomy Registry (deployed in a simultaneous proposal with this one), adds the ability to do limits and stops on Osmosis.

Wrapper is literally a wrapper for Osmosis swap operation.
Swap msg includes the input for the swap as well as the output check params.
Wrapper reverts if the output amount is not between `min` and `max`.

### Structure

    .
    ├── contracts                   # Smart Contacts
    ├── packages                    # Hardhat helpers for test and deploy
    ├── integration-tests           # Scripts for integration test
    ├── proposals                   # Proposals for mainnet
    ├── ...                         # Config and other files
    └── README.md

### Compile contracts

Launch the docker for `workspace-optimizer:0.12.8` and use beaker to build wasm contracts.

```bash
beaker wasm build
```

Use `cargo wasm` and `optimize.sh` if that's what you like.

### Test contracts

- Run LocalOsmosis node by using [https://github.com/osmosis-labs/LocalOsmosis](https://github.com/osmosis-labs/LocalOsmosis)
- Setup contracts and run the test

```bash
cd ./integration-tests
yarn test:localosmosis-setup
yarn test:localosmosis-test-wrapper
yarn test:localosmosis-test-registry
```

Explore more inside integration-tests.
