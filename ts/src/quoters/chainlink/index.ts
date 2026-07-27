import { canonicalizeAddress, canonicalizeAsset } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParameters, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import { mulDiv, pow10 } from "../../utils/math.js";
import * as abi from "./abi.js";

export type ChainlinkQuoterParams = {
  networkId: number;
  feedAddress: `0x${string}`;
  token: string;
  quote: string;
  feedDecimals: number;
  tokenDecimals: number;
  quoteDecimals: number;
  confidence?: number;
};

export const chainlinkQuoter = (parameters: ChainlinkQuoterParams): Quoter => ({
  identity: `chainlink:${canonicalizeAddress(parameters.feedAddress)}:${parameters.token}:${parameters.quote}`,
  assets: [canonicalizeAsset(parameters.token), canonicalizeAsset(parameters.quote)],
  confidence: parameters.confidence ?? 0,
  quote: async ({ amountIn, direction, context }: QuoteParameters) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    const round = await contractCall(context.getProvider(parameters.networkId), parameters.feedAddress, abi.latestRoundData, [], context.blockNumber);

    const answer = readAnswer(round);

    if (answer === undefined) throw new EthPricesError("CONTRACT_ERROR", "Chainlink returned malformed round data");

    if (answer < 0n) throw new EthPricesError("RATE_UNAVAILABLE", "Chainlink feed returned a negative answer");

    const rate = answer;
    const tokenScale = pow10(parameters.tokenDecimals);
    const rateScale = pow10(parameters.feedDecimals);
    const quoteScale = pow10(parameters.quoteDecimals);

    return direction === "forward"
      ? mulDiv(amountIn, rate * quoteScale, rateScale * tokenScale)
      : mulDiv(amountIn, rateScale * tokenScale, rate * quoteScale);
  },
});

const readAnswer = (value: unknown): bigint | undefined => {
  if (Array.isArray(value) && typeof value[1] === "bigint") return value[1];

  if (typeof value === "object" && value !== null && "answer" in value && typeof value.answer === "bigint") return value.answer;

  return undefined;
};
