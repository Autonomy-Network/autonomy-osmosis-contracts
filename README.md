## Autonomy Network for Osmosis

Autonomy Network is a decentralized automation protocol - it allows you to have transactions automatically execute in the future under arbitrary conditions.
It's an infrastructure service, and a tool, that allows anyone to automate any on-chain action with any on-chain condition. For example, limit orders/stop losses for traders of decentralized exchanges like Uniswap, and exit liquidity positions once impermanent loss becomes too great. A blog post giving a simple overview of Autonomy and each component can be found here. Autonomy is full composable, so it can even be used by on-chain entities like DAOs to automate things such as recurring salary payments. Because it's infrastructure that's meant to be used by other projects directly to add features, similar to ChainLink, users don't even need to know it exists - they just use a dapp that automates something, such as a limit order, and the experience is the same as it would be with a centralized exchange. Users don't need to learn anything or deploy a contract, it 'just works'!
Autonomy is ultimately a B2B tool. However, the team has been creating user-facing dapps to prove the feasibility and demand for new use cases, using them as a stepping stone for the automation features being integrated into other projects natively or having others fork our dapps and improve them while still using Autonomy under the hood.

### Contracts for Osmosis

Osmosis contracts consist of Registry and Wrapper.

- Registry stores requests. Users can stake their AUTO tokens into Registry in order to execute requests for their epoch. Things are generally automated by using the Autonomy Registry in conjunction with a wrapper contract around some system - for example like a wrapper around the Osmosis DEX that allows for limits and stops on Osmosis. A 2nd simultaneous proposal will deploy this wrapper contract to add limits and stops to Osmosis.
- Wrapper contract executes the swap operation between assets and validate results, when used with the Autonomy Registry (deployed in a simultaneous proposal with this one), adds the ability to do limits and stops on Osmosis.

### Structure

    .
    ├── contracts                   # Smart Contacts
    ├── packages                    # Hardhat helpers for test and deploy
    ├── integration-tests           # Scripts for integration test
    ├── proposals                   # Proposals for mainnet
    ├── ...                         # Config and other files
    └── README.md

### Compile contracts

Launch the docker for `rust-optimizer:0.12.8` and use beaker to build wasm contracts.

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
