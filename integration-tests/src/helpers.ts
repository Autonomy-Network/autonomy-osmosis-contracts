import * as fs from "fs";
import chalk from "chalk";
import BN from "bn.js";
import axios from "axios";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { localosmosis } from "./config/localosmosisConstants";

/**
 * @notice Encode a JSON object to base64 binary
 */
export function toEncodedBinary(obj: any): string {
  return Buffer.from(JSON.stringify(obj)).toString("base64");
}

export function datetimeStringToUTC(date: string): number {
  try {
    return Math.round(Date.parse(date) / 1000);
  } catch (err) {
    throw "Date given is not parsable";
  }
}


/**
 * @notice Upload contract code to LocalOsmosis. Return code ID.
 */
export async function storeCode(
  client: SigningCosmWasmClient,
  deployer: DirectSecp256k1HdWallet,
  filepath: string
): Promise<number> {
  const code = fs.readFileSync(filepath);
  // const [account] = await deployer.getAccounts();

  const result = await client.upload(localosmosis.addresses.wallet1, code, "auto", );
  return result.codeId;
}

/**
 * @notice Instantiate a contract from an existing code ID. Return contract address.
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export async function instantiateContract(
  client: SigningCosmWasmClient,
  deployer: DirectSecp256k1HdWallet,
  admin: DirectSecp256k1HdWallet, // leave this emtpy then contract is not migratable
  codeId: number,
  instantiateMsg: Record<string, unknown>
) {
  // const [account] = await deployer.getAccounts();
  const result = await client.instantiate(localosmosis.addresses.wallet1, codeId, instantiateMsg, "instantiate", "auto");
  return result;
}

/**
 * @notice Instantiate a contract from an existing code ID. Return contract address.
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export async function migrateContract(
  client: SigningCosmWasmClient,
  sender: DirectSecp256k1HdWallet,
  admin: DirectSecp256k1HdWallet,
  contract: string,
  new_code_id: number,
  migrateMsg: Record<string, unknown>
) {
  // const [account] = await sender.getAccounts();
  const result = await client.migrate(localosmosis.addresses.wallet1, contract, new_code_id, migrateMsg, "auto");
  return result;
}
