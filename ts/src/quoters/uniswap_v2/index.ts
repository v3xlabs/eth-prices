import { canonicalizeAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams as QuoteParameters, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import { mulDiv } from "../../utils/math.js";
import * as abi from "./abi.js";

export type UniswapV2QuoterParams = {
  networkId: number;
  pairAddress: `0x${string}`;
  token0: `0x${string}`;
  token1: `0x${string}`;
  confidence?: number;
  identityPrefix?: string;
};

export const uniswapV2Quoter = (parameters: UniswapV2QuoterParams): Quoter => ({
  identity: `${parameters.identityPrefix ?? "uniswap_v2"}:${canonicalizeAddress(parameters.pairAddress)}`,
  assets: [canonicalizeAddress(parameters.token0), canonicalizeAddress(parameters.token1)],
  confidence: parameters.confidence ?? 0,
  quote: async ({ amountIn, direction, context }: QuoteParameters) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    const reserves = await contractCall(context.getProvider(parameters.networkId), parameters.pairAddress, abi.getReserves, [], context.blockNumber);

    const normalized = normalizeReserves(reserves);

    if (normalized === undefined) throw new EthPricesError("CONTRACT_ERROR", "Uniswap V2 returned malformed reserves");

    const { reserve0, reserve1 } = normalized;

    return direction === "forward" ? mulDiv(amountIn, reserve1, reserve0) : mulDiv(amountIn, reserve0, reserve1);
  },
});

const normalizeReserves = (value: unknown): { reserve0: bigint; reserve1: bigint; } | undefined => {
  if (Array.isArray(value) && typeof value[0] === "bigint" && typeof value[1] === "bigint") {
    return { reserve0: value[0], reserve1: value[1] };
  }

  if (typeof value === "object" && value !== null && "reserve0" in value && "reserve1" in value
    && typeof value.reserve0 === "bigint" && typeof value.reserve1 === "bigint") {
    return { reserve0: value.reserve0, reserve1: value.reserve1 };
  }

  return undefined;
};
