import { canonicalizeAsset } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { mulDiv, pow10 } from "../../utils/math.js";

export type FixedQuoterParams = {
  inputAsset: string;
  inputAssetDecimals: number;
  outputAsset: string;
  outputAssetDecimals: number;
  fixedRate: bigint;
  fixedRateDecimals: number;
  confidence?: number;
};

export const fixedQuoter = (params: FixedQuoterParams): Quoter => ({
  identity: `fixed:${params.inputAsset}:${params.outputAsset}`,
  assets: [canonicalizeAsset(params.inputAsset), canonicalizeAsset(params.outputAsset)],
  confidence: params.confidence ?? 0,
  quote: async ({ amountIn, direction }: QuoteParams) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    if (params.fixedRate < 0n) throw new EthPricesError("INVALID_CONFIGURATION", "fixedRate must not be negative");

    const inputAssetScale = pow10(params.inputAssetDecimals);
    const outputAssetScale = pow10(params.outputAssetDecimals);
    const rateScale = pow10(params.fixedRateDecimals);

    return direction === "forward"
      ? mulDiv(amountIn, params.fixedRate * outputAssetScale, rateScale * inputAssetScale)
      : mulDiv(amountIn, rateScale * inputAssetScale, params.fixedRate * outputAssetScale);
  },
});
