import { checksum, from } from "ox/Address";

import { EthPricesError } from "./error.js";

export type EvmAddress = `0x${string}`;
export type AssetIdentifier = string;

export const canonicalizeAddress = (value: string): EvmAddress => {
  try {
    return checksum(from(value));
  }
  catch (error: unknown) {
    throw new EthPricesError("INVALID_ADDRESS", `Invalid address: ${value}`, { cause: error });
  }
};

export const canonicalizeAsset = (value: AssetIdentifier): AssetIdentifier => {
  if (value.startsWith("0x")) return canonicalizeAddress(value);

  return value;
};

export const isEvmAddress = (value: AssetIdentifier): value is EvmAddress => {
  try {
    canonicalizeAddress(value);

    return value.startsWith("0x");
  }
  catch {
    return false;
  }
};
