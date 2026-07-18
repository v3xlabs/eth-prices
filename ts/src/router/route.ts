import { EthPricesError } from "../error.js";
import type { QuoteParams, RouteStep } from "../quoter.js";

export type Route = {
  path: readonly RouteStep[];
  inputAsset: string;
  outputAsset: string;
};

export const quoteRoute = async (route: Route, params: Omit<QuoteParams, "direction">): Promise<bigint> => {
  if (params.amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

  let amountOut = params.amountIn;

  for (const step of route.path) {
    amountOut = await step.quoter.quote({
      ...params,
      amountIn: amountOut,
      direction: step.direction,
    });
  }

  return amountOut;
};
