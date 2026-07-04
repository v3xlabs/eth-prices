import { canonicalizeAsset } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { mulDiv, pow10 } from "../../utils/math.js";

export type FixedQuoterParams = {
  assetIn: string;
  assetInDecimals: number;
  assetOut: string;
  assetOutDecimals: number;
  fixedRate: bigint;
  fixedRateDecimals: number;
  confidence?: number;
};

export const fixedQuoter = (params: FixedQuoterParams): Quoter => ({
  identity: `fixed:${params.assetIn}:${params.assetOut}`,
  assets: [canonicalizeAsset(params.assetIn), canonicalizeAsset(params.assetOut)],
  confidence: params.confidence ?? 0,
  quote: async ({ amountIn, direction }: QuoteParams) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    if (params.fixedRate < 0n) throw new EthPricesError("INVALID_CONFIGURATION", "fixedRate must not be negative");

    const tokenInScale = pow10(params.assetInDecimals);
    const tokenOutScale = pow10(params.assetOutDecimals);
    const rateScale = pow10(params.fixedRateDecimals);

    return direction === "forward"
      ? mulDiv(amountIn, params.fixedRate * tokenOutScale, rateScale * tokenInScale)
      : mulDiv(amountIn, rateScale * tokenInScale, params.fixedRate * tokenOutScale);
  },
});
