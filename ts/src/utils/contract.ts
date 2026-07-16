import { type AbiFunction, decodeResult, encodeData } from "ox/AbiFunction";
import { fromNumber as hexFromNumber } from "ox/Hex";

import { EthPricesError } from "../error.js";

export type RpcProvider = {
  request(params: { method: string; params?: unknown[]; }): Promise<unknown>;
};

export const contractCall = async (
  provider: RpcProvider,
  address: `0x${string}`,
  fn: AbiFunction,
  args: readonly unknown[] = [],
  blockNumber?: bigint,
): Promise<unknown> => {
  if (provider === undefined) throw new EthPricesError("INVALID_NETWORK", "A provider is required");

  const data = encodeData(fn, args);
  const blockParam = blockNumber === undefined
    ? "latest"
    : hexFromNumber(blockNumber);

  try {
    const result = await provider.request({
      method: "eth_call",
      params: [{ to: address.toLowerCase(), data }, blockParam],
    });

    if (!isHex(result)) {
      throw new EthPricesError("CONTRACT_ERROR", "RPC returned malformed eth_call data");
    }

    return decodeResult(fn, result);
  }
  catch (error: unknown) {
    if (error instanceof EthPricesError) throw error;

    throw new EthPricesError("CONTRACT_ERROR", `Contract call failed for ${address}`, { cause: error });
  }
};

const isHex = (value: unknown): value is `0x${string}` => typeof value === "string" && value.startsWith("0x");
