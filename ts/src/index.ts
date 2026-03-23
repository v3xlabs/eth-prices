export type BlockNumberish = number | bigint;
import { createQuoter as createWasmQuoter } from "../pkg/eth_prices.js";
import type {
  CreateQuoterConfig,
  FixedQuoterConfig,
  QuoteRequest,
  UniswapV2Selector,
  UniswapV3Selector,
  WasmQuoter,
  WasmRoute,
} from "../pkg/eth_prices.js";

export type { CreateQuoterConfig, FixedQuoterConfig, QuoteRequest, UniswapV2Selector, UniswapV3Selector };
export type TokenIdentifier = string;

export class Quoter {
  private constructor(private readonly inner: WasmQuoter) {}

  static async create(config: CreateQuoterConfig): Promise<Quoter> {
    const inner = (await createWasmQuoter(config)) as WasmQuoter;
    return new Quoter(inner);
  }

  addFixedQuoter(config: FixedQuoterConfig): void {
    this.inner.addFixedQuoter(config);
  }

  addUniswapV2Quoter(selector: UniswapV2Selector): Promise<void> {
    return this.inner.addUniswapV2Quoter(selector);
  }

  addUniswapV3Quoter(selector: UniswapV3Selector): Promise<void> {
    return this.inner.addUniswapV3Quoter(selector);
  }

  addErc4626Quoter(vaultAddress: string): Promise<void> {
    return this.inner.addErc4626Quoter(vaultAddress);
  }

  computeRoute(inputToken: TokenIdentifier, outputToken: TokenIdentifier): WasmRoute {
    return this.inner.computeRoute(inputToken, outputToken);
  }

  quoteRoute(route: WasmRoute, amountIn: string, block?: BlockNumberish): Promise<string> {
    return this.inner.quoteRoute(route, amountIn, toWasmBlock(block));
  }

  getRate(
    inputToken: TokenIdentifier,
    outputToken: TokenIdentifier,
    amountIn: string,
    block?: BlockNumberish,
  ): Promise<string> {
    return this.inner.getRate(inputToken, outputToken, amountIn, toWasmBlock(block));
  }

  quote(request: QuoteRequest): Promise<string> {
    return this.inner.quote({
      ...request,
      block: toWasmBlock(request.block),
    });
  }

  getLatestBlock(): Promise<bigint> {
    return this.inner.getLatestBlock();
  }

  listQuoters(): string[] {
    return this.inner.listQuoters();
  }
}

export const createQuoter = Quoter.create;

function toWasmBlock(block?: BlockNumberish): bigint | undefined {
  if (block === undefined) {
    return undefined;
  }

  return typeof block === "bigint" ? block : BigInt(block);
}
