import { canonicalizeAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import * as abi from "./abi.js";

export type CurvePoolKind = "stableswap" | "crypto";

export type CurveQuoterParams = {
  networkId: number;
  poolAddress: `0x${string}`;
  token0: `0x${string}`;
  token1: `0x${string}`;
  coinIndex0: number;
  coinIndex1: number;
  kind: CurvePoolKind;
  confidence?: number;
  identityPrefix?: string;
};

export const curveQuoter = (params: CurveQuoterParams): Quoter => {
  assertCoinIndex(params.coinIndex0, "coinIndex0");
  assertCoinIndex(params.coinIndex1, "coinIndex1");

  return {
    identity: `${params.identityPrefix ?? "curve"}:${canonicalizeAddress(params.poolAddress)}:${params.coinIndex0}-${params.coinIndex1}`,
    assets: [canonicalizeAddress(params.token0), canonicalizeAddress(params.token1)],
    confidence: params.confidence ?? 0,
    quote: async ({ amountIn, direction, context }: QuoteParams) => {
      if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

      const [coinIn, coinOut] = direction === "forward"
        ? [params.coinIndex0, params.coinIndex1]
        : [params.coinIndex1, params.coinIndex0];
      const getDy = params.kind === "stableswap" ? abi.getDyStableSwap : abi.getDyCrypto;
      const result = await contractCall(
        context.getProvider(params.networkId),
        params.poolAddress,
        getDy,
        [BigInt(coinIn), BigInt(coinOut), amountIn],
        context.blockNumber,
      );

      if (typeof result !== "bigint") throw new EthPricesError("CONTRACT_ERROR", "Curve pool returned malformed get_dy data");

      return result;
    },
  };
};

const assertCoinIndex = (value: number, name: string): void => {
  if (!Number.isSafeInteger(value) || value < 0 || value > 7) {
    throw new EthPricesError("INVALID_CONFIGURATION", `${name} must be a pool coin index between 0 and 7`);
  }
};
