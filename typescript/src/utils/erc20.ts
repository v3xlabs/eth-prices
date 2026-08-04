import { type AbiFunction, from } from "ox/AbiFunction";

import type { EvmAddress } from "../asset.js";
import { settleMap } from "./concurrency.js";
import { contractCall, decodedUint, type RpcProvider } from "./contract.js";

export const decimalsAbi: AbiFunction = from({
  name: "decimals",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "uint8" }],
  stateMutability: "view",
});

const FALLBACK_DECIMALS = 18;

export const fetchDecimals = async (
  provider: RpcProvider,
  addresses: readonly EvmAddress[],
  blockNumber?: bigint,
  concurrency = 16,
): Promise<ReadonlyMap<string, number>> => {
  const unique = new Map<string, EvmAddress>();

  for (const address of addresses) unique.set(address.toLowerCase(), address);

  const settled = await settleMap([...unique.values()], concurrency, async (address) => {
    const value = decodedUint(await contractCall(provider, address, decimalsAbi, [], blockNumber));

    return value !== undefined && value <= 255 ? value : undefined;
  });
  const decimals = new Map<string, number>();

  for (const { input, result } of settled) {
    decimals.set(
      input.toLowerCase(),
      result.status === "fulfilled" && result.value !== undefined ? result.value : FALLBACK_DECIMALS,
    );
  }

  return decimals;
};

export const decimalsOf = (decimals: ReadonlyMap<string, number>, address: EvmAddress): number =>
  decimals.get(address.toLowerCase()) ?? FALLBACK_DECIMALS;
