import { EthPricesError } from "../error.js";
import type { QuoteParameters, RouteStep } from "../quoter.js";

export type Route = {
  path: readonly RouteStep[];
  inputAsset: string;
  outputAsset: string;
};

export const quoteRoute = async (route: Route, parameters: Omit<QuoteParameters, "direction">): Promise<bigint> => {
  if (parameters.amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

  let amountOut = parameters.amountIn;

  for (const step of route.path) {
    amountOut = await step.quoter.quote({
      ...parameters,
      amountIn: amountOut,
      direction: step.direction,
    });
  }

  return amountOut;
};
