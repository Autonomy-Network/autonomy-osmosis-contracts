# Autonomy Integration Tests

## Requirements

- docker
- [LocalOsmosis](https://github.com/osmosis-labs/LocalOsmosis)

## Procedures

### Start LocalOsmosis

```bash
git clone https://github.com/osmosis-labs/LocalOsmosis.git
cd LocalOsmosis
```

Once done, start LocalOsmosis by

```bash
docker-compose up  # Ctrl + C to quit
```

(For details, please reference [Osmosis Doc](https://docs.osmosis.zone/cosmwasm/local/localosmosis#setup--localosmosis))

When you may need to revert LocalOsmosis to its initial state, run

```bash
docker-compose rm
```

### Compile contracts

```bash
# .zshrc or .bashrc
# set the optimizer version to whichever latest version of optimizer (currently it is 0.12.5):
alias workspace-optimizer='docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.8'
```

```bash
# from the root folder in the autonomy-osmosis-program repo
workspace-optimizer
```

### Create and configure wasms paths file

You need to tell the test suite where to find the wasms artifacts files locally for the various repos it works with.


First, copy the built wasm files into the `./src/config/wasms` dir of this repo.

In the `src/config` folder there is an example file for setting the parameters that point to your local wasm folders: `wasmPaths.ts.example`
In the newly created file, edit the `wasm_path` object's attributes for the `station` to point to the `./src/config/wasms` dir.

```bash
cp ./src/config/wasmPaths.ts.example ./src/config/wasmPaths.ts
nano ./src/config/wasmPaths.ts
```

### LocalOsmosis constants file setup

In the `src/config` folder there is an example file for setting the constants for your LocalOsmosis parameters (contracts, wallets, etc): `localosmosisConstants.ts.example`

```bash
cp ./src/config/localosmosisConstants.ts.example ./src/config/localosmosisConstants.ts
nano ./src/config/localosmosisConstants.ts
```

### Run full setup of contracts & all tests

```bash
yarn
yarn test:localosmosis-setup
yarn test:localosmosis-test-wrapper
yarn test:localosmosis-test-registry
```

**NOTE:** After each of the setup commands, you may see key contract addresses or wasm codes that will need to updated in your `localosmosisConstants.ts` file before proceeding to run the next command. These commands build upon on another.
Also, after one command, the terminal does not automatically get back. So, you should do it manually by `Ctrl + C`.
