import { canonicalizeAddress, type EvmAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { Discoverer, DiscoveryFailure } from "../../router/discovery.js";
import { failureMessage } from "../../router/discovery.js";
import { settleMap } from "../../utils/concurrency.js";
import { contractCall } from "../../utils/contract.js";
import * as poolAbi from "./abi.js";
import { getPool } from "./factoryAbi.js";
import { uniswapV3Quoter } from "./index.js";

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";
const CONFIDENCE_DIVISOR = 10_000_000_000_000_000n;
const DEFAULT_FEES: readonly number[] = [100, 500, 3000, 10_000];

export type UniswapV3DiscovererOptions = {
  readonly networkId: number;
  readonly factoryAddress: EvmAddress;
  readonly identity?: string;
  readonly fees?: readonly number[];
  readonly minLiquidity?: bigint;
  readonly concurrency?: number;
};

type Pool = {
  readonly poolAddress: EvmAddress;
  readonly token0: EvmAddress;
  readonly token1: EvmAddress;
  readonly fee: number;
  readonly liquidity: bigint;
};

type Query = readonly [EvmAddress, EvmAddress, number];

export const uniswapV3Discoverer = (options: UniswapV3DiscovererOptions): Discoverer => {
  const factoryAddress = canonicalizeAddress(options.factoryAddress);
  const identity = options.identity ?? "uniswap_v3";
  const fees = options.fees ?? DEFAULT_FEES;
  const minLiquidity = options.minLiquidity ?? 1n;

  for (const fee of fees) {
    if (!Number.isSafeInteger(fee) || fee < 0 || fee > 0xFF_FF_FF) {
      throw new EthPricesError("INVALID_CONFIGURATION", `Invalid V3 fee: ${fee}`);
    }
  }

  return {
    identity: `${identity}:${factoryAddress}`,
    discover: async ({ tokens, context }) => {
      const queries = poolQueries(tokens, fees);
      const provider = context.getProvider(options.networkId);
      const settled = await settleMap(queries, options.concurrency ?? 16, async ([tokenA, tokenB, fee]): Promise<Pool | undefined> => {
        const pool = await contractCall(provider, factoryAddress, getPool, [tokenA, tokenB, BigInt(fee)], context.blockNumber);

        if (!isAddress(pool)) throw new TypeError("V3 factory returned malformed pool address");

        if (pool.toLowerCase() === ZERO_ADDRESS) return undefined;

        const poolAddress = canonicalizeAddress(pool);
        const [token0Result, token1Result, liquidityResult] = await Promise.all([
          contractCall(provider, poolAddress, poolAbi.token0, [], context.blockNumber),
          contractCall(provider, poolAddress, poolAbi.token1, [], context.blockNumber),
          contractCall(provider, poolAddress, poolAbi.liquidity, [], context.blockNumber),
        ]);

        if (!isAddress(token0Result) || !isAddress(token1Result) || typeof liquidityResult !== "bigint") {
          throw new TypeError("V3 pool returned malformed pool data");
        }

        return {
          poolAddress,
          token0: canonicalizeAddress(token0Result),
          token1: canonicalizeAddress(token1Result),
          fee,
          liquidity: liquidityResult,
        };
      });
      const failures: DiscoveryFailure[] = [];
      const pools: Pool[] = [];
      let skipped = 0;

      for (const { input: [tokenA, tokenB, fee], result } of settled) {
        if (result.status === "rejected") {
          failures.push({ target: `${tokenA}/${tokenB}/${fee}`, message: failureMessage(result.reason), cause: result.reason });
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
      const quoters = bestPools.map(pool => uniswapV3Quoter({
        networkId: options.networkId,
        poolAddress: pool.poolAddress,
        token0: pool.token0,
        token1: pool.token1,
        confidence: confidence(pool.liquidity),
        identityPrefix: identity,
      }));

      return { quoters, attempted: queries.length, skipped, failures };
    },
  };
};

const poolQueries = (tokens: readonly EvmAddress[], fees: readonly number[]): Query[] => {
  const queries: Query[] = [];

  for (const [left, tokenA] of tokens.entries()) {
    for (const tokenB of tokens.slice(left + 1)) {
      for (const fee of fees) queries.push([tokenA, tokenB, fee]);
    }
  }

  return queries;
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
const isAddress = (value: unknown): value is EvmAddress => typeof value === "string" && value.startsWith("0x");
