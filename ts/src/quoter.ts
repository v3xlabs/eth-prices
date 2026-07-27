import type { AssetIdentifier } from "./asset.js";
import type { NetworkContext } from "./network.js";

export type { AssetIdentifier } from "./asset.js";

export type Direction = "forward" | "reverse";

export type QuoteParameters = {
  amountIn: bigint;
  direction: Direction;
  context: NetworkContext;
};

export type Quote = {
  amountIn: bigint;
  amountOut: bigint;
  assetIn: AssetIdentifier;
  assetOut: AssetIdentifier;
  quoter: string;
};

export type Quoter = {
  readonly identity: string;
  readonly assets: readonly [AssetIdentifier, AssetIdentifier];
  readonly confidence: number;
  quote(parameters: QuoteParameters): Promise<bigint>;
};

export type RouteStep = {
  quoter: Quoter;
  direction: Direction;
};
