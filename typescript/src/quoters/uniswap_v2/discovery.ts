import { canonicalizeAddress, type EvmAddress } from "../../asset.js";
import type { Discoverer, DiscoveryFailure } from "../../router/discovery.js";
import { failureMessage } from "../../router/discovery.js";
import { settleMap } from "../../utils/concurrency.js";
import { contractCall, decodedUint, fetchBlockTimestamp } from "../../utils/contract.js";
import { decimalsOf, fetchDecimals } from "../../utils/erc20.js";
import { freshnessMultiplier, liquidityConfidence } from "../../utils/math.js";
import * as pairAbi from "./abi.js";
import { uniswapV2Quoter } from "./index.js";

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";

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
  readonly reserve0: bigint;
  readonly reserve1: bigint;
  readonly liquidity: bigint;
  readonly lastTradeTimestamp: number | undefined;
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
          reserve0: reserves.reserve0,
          reserve1: reserves.reserve1,
          liquidity: minBigInt(reserves.reserve0, reserves.reserve1),
          lastTradeTimestamp: reserves.blockTimestampLast,
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
      const [decimals, blockTimestamp] = await Promise.all([
        fetchDecimals(
          provider,
          bestPools.flatMap(pool => [pool.token0, pool.token1]),
          context.blockNumber,
          options.concurrency ?? 16,
        ),
        fetchBlockTimestamp(provider, context.blockNumber),
      ]);
      const quoters = bestPools.map(pool => uniswapV2Quoter({
        networkId: options.networkId,
        pairAddress: pool.pairAddress,
        token0: pool.token0,
        token1: pool.token1,
        confidence: confidence(pool, decimals, blockTimestamp),
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

const confidence = (pool: Pool, decimals: ReadonlyMap<string, number>, blockTimestamp: number): number => {
  const units0 = Number(pool.reserve0) / 10 ** decimalsOf(decimals, pool.token0);
  const units1 = Number(pool.reserve1) / 10 ** decimalsOf(decimals, pool.token1);
  const freshness = pool.lastTradeTimestamp === undefined
    ? 1
    : freshnessMultiplier(blockTimestamp - pool.lastTradeTimestamp);

  return Math.round(liquidityConfidence(Math.sqrt(units0 * units1)) * freshness);
};
const minBigInt = (left: bigint, right: bigint): bigint => (left < right ? left : right);
const isAddress = (value: unknown): value is EvmAddress => typeof value === "string" && value.startsWith("0x");
const normalizeReserves = (value: unknown): { reserve0: bigint; reserve1: bigint; blockTimestampLast: number | undefined; } | undefined => {
  if (Array.isArray(value) && typeof value[0] === "bigint" && typeof value[1] === "bigint") {
    return { reserve0: value[0], reserve1: value[1], blockTimestampLast: decodedUint(value[2]) };
  }

  if (typeof value === "object" && value !== null && "reserve0" in value && "reserve1" in value
    && typeof value.reserve0 === "bigint" && typeof value.reserve1 === "bigint") {
    return {
      reserve0: value.reserve0,
      reserve1: value.reserve1,
      blockTimestampLast: "blockTimestampLast" in value ? decodedUint(value.blockTimestampLast) : undefined,
    };
  }

  return undefined;
};
