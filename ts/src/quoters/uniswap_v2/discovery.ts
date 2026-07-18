import { canonicalizeAddress, type EvmAddress } from "../../asset.js";
import type { Discoverer, DiscoveryFailure } from "../../router/discovery.js";
import { failureMessage } from "../../router/discovery.js";
import { settleMap } from "../../utils/concurrency.js";
import { contractCall } from "../../utils/contract.js";
import * as pairAbi from "./abi.js";
import { uniswapV2Quoter } from "./index.js";

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";
const CONFIDENCE_DIVISOR = 1_000_000_000n;

export type UniswapV2DiscovererOptions = {
  readonly networkId: number;
  readonly factoryAddress: EvmAddress;
  readonly identity?: string;
  readonly minLiquidity?: bigint;
  readonly concurrency?: number;
};

type Pool = {
  readonly pairAddress: EvmAddress;
  readonly token0: EvmAddress;
  readonly token1: EvmAddress;
  readonly liquidity: bigint;
};

export const uniswapV2Discoverer = (options: UniswapV2DiscovererOptions): Discoverer => {
  const factoryAddress = canonicalizeAddress(options.factoryAddress);
  const identity = options.identity ?? "uniswap_v2";
  const minLiquidity = options.minLiquidity ?? 1n;

  return {
    identity: `${identity}:${factoryAddress}`,
    discover: async ({ tokens, context }) => {
      const pairs = tokenPairs(tokens);
      const provider = context.getProvider(options.networkId);
      const settled = await settleMap(pairs, options.concurrency ?? 16, async ([tokenA, tokenB]): Promise<Pool | undefined> => {
        const pair = await contractCall(provider, factoryAddress, pairAbi.factoryGetPair, [tokenA, tokenB], context.blockNumber);

        if (!isAddress(pair)) throw new TypeError("V2 factory returned malformed pair address");

        if (pair.toLowerCase() === ZERO_ADDRESS) return undefined;

        const pairAddress = canonicalizeAddress(pair);
        const [token0Result, token1Result, reservesResult] = await Promise.all([
          contractCall(provider, pairAddress, pairAbi.token0, [], context.blockNumber),
          contractCall(provider, pairAddress, pairAbi.token1, [], context.blockNumber),
          contractCall(provider, pairAddress, pairAbi.getReserves, [], context.blockNumber),
        ]);

        const reserves = normalizeReserves(reservesResult);

        if (!isAddress(token0Result) || !isAddress(token1Result) || reserves === undefined) {
          throw new TypeError("V2 pair returned malformed pool data");
        }

        return {
          pairAddress,
          token0: canonicalizeAddress(token0Result),
          token1: canonicalizeAddress(token1Result),
          liquidity: minBigInt(reserves.reserve0, reserves.reserve1),
        };
      });
      const failures: DiscoveryFailure[] = [];
      const pools: Pool[] = [];
      let skipped = 0;

      for (const { input: [tokenA, tokenB], result } of settled) {
        if (result.status === "rejected") {
          failures.push({ target: `${tokenA}/${tokenB}`, message: failureMessage(result.reason), cause: result.reason });
        }
        else if (result.value === undefined || result.value.liquidity < minLiquidity) {
          skipped += 1;
        }
        else {
          pools.push(result.value);
        }
      }

      const bestPools = deduplicatePools(pools);

      skipped += pools.length - bestPools.length;
      const quoters = bestPools.map(pool => uniswapV2Quoter({
        networkId: options.networkId,
        pairAddress: pool.pairAddress,
        token0: pool.token0,
        token1: pool.token1,
        confidence: confidence(pool.liquidity),
        identityPrefix: identity,
      }));

      return { quoters, attempted: pairs.length, skipped, failures };
    },
  };
};

const tokenPairs = (tokens: readonly EvmAddress[]): Array<readonly [EvmAddress, EvmAddress]> => {
  const pairs: Array<readonly [EvmAddress, EvmAddress]> = [];

  for (const [left, tokenA] of tokens.entries()) {
    for (const tokenB of tokens.slice(left + 1)) pairs.push([tokenA, tokenB]);
  }

  return pairs;
};

const deduplicatePools = (pools: readonly Pool[]): Pool[] => {
  const best = new Map<string, Pool>();

  for (const pool of pools) {
    const key = [pool.token0.toLowerCase(), pool.token1.toLowerCase()].sort().join(":");
    const existing = best.get(key);

    if (existing === undefined || pool.liquidity > existing.liquidity) best.set(key, pool);
  }

  return [...best.values()];
};

const confidence = (liquidity: bigint): number => Number(liquidity / CONFIDENCE_DIVISOR > 100n ? 100n : liquidity / CONFIDENCE_DIVISOR);
const minBigInt = (left: bigint, right: bigint): bigint => (left < right ? left : right);
const isAddress = (value: unknown): value is EvmAddress => typeof value === "string" && value.startsWith("0x");
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
