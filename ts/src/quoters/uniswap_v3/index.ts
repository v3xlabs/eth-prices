import { canonicalizeAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import { mulDiv } from "../../utils/math.js";
import * as abi from "./abi.js";

export type UniswapV3QuoterParams = {
  networkId: number;
  poolAddress: `0x${string}`;
  token0: `0x${string}`;
  token1: `0x${string}`;
  confidence?: number;
  identityPrefix?: string;
};

export const uniswapV3Quoter = (params: UniswapV3QuoterParams): Quoter => ({
  identity: `${params.identityPrefix ?? "uniswap_v3"}:${canonicalizeAddress(params.poolAddress)}`,
  assets: [canonicalizeAddress(params.token0), canonicalizeAddress(params.token1)],
  confidence: params.confidence ?? 0,
  quote: async ({ amountIn, direction, context }: QuoteParams) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    const slot = await contractCall(context.getProvider(params.networkId), params.poolAddress, abi.slot0, [], context.blockNumber);

    const sqrtPriceX96 = readSqrtPrice(slot);

    if (sqrtPriceX96 === undefined) throw new EthPricesError("CONTRACT_ERROR", "Uniswap V3 returned malformed slot0 data");

    const q192 = 1n << 192n;
    const squaredPrice = sqrtPriceX96 * sqrtPriceX96;

    return direction === "forward" ? mulDiv(amountIn, squaredPrice, q192) : mulDiv(amountIn, q192, squaredPrice);
  },
});

const readSqrtPrice = (value: unknown): bigint | undefined => {
  if (Array.isArray(value) && typeof value[0] === "bigint") return value[0];

  if (typeof value === "object" && value !== null && "sqrtPriceX96" in value && typeof value.sqrtPriceX96 === "bigint") {
    return value.sqrtPriceX96;
  }

  return undefined;
};
