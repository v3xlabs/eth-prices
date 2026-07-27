import { canonicalizeAsset } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams as QuoteParameters, Quoter } from "../../quoter.js";
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

export const fixedQuoter = (parameters: FixedQuoterParams): Quoter => ({
  identity: `fixed:${parameters.inputAsset}:${parameters.outputAsset}`,
  assets: [canonicalizeAsset(parameters.inputAsset), canonicalizeAsset(parameters.outputAsset)],
  confidence: parameters.confidence ?? 0,
  quote: async ({ amountIn, direction }: QuoteParameters) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    if (parameters.fixedRate < 0n) throw new EthPricesError("INVALID_CONFIGURATION", "fixedRate must not be negative");

    const inputAssetScale = pow10(parameters.inputAssetDecimals);
    const outputAssetScale = pow10(parameters.outputAssetDecimals);
    const rateScale = pow10(parameters.fixedRateDecimals);

    return direction === "forward"
      ? mulDiv(amountIn, parameters.fixedRate * outputAssetScale, rateScale * inputAssetScale)
      : mulDiv(amountIn, rateScale * inputAssetScale, parameters.fixedRate * outputAssetScale);
  },
});
