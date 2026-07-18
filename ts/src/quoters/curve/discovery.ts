import { canonicalizeAddress, type EvmAddress } from "../../asset.js";
import type { Discoverer, DiscoveryFailure } from "../../router/discovery.js";
import { failureMessage } from "../../router/discovery.js";
import { settleMap } from "../../utils/concurrency.js";
import { contractCall, decodedUint, type RpcProvider } from "../../utils/contract.js";
import { decimalsOf, fetchDecimals } from "../../utils/erc20.js";
import { liquidityConfidence, pow10 } from "../../utils/math.js";
import * as abi from "./abi.js";
import { type CurvePoolKind, curveQuoter } from "./index.js";

export const CURVE_META_REGISTRY_MAINNET: EvmAddress = "0xF98B45FA17DE75FB1aD0e7aFD971b0ca00e379fC";

export type CurveDiscovererOptions = {
  readonly networkId: number;
  readonly metaRegistryAddress?: EvmAddress;
  readonly identity?: string;
  readonly minLiquidity?: bigint;
  readonly concurrency?: number;
};

type Pool = {
  readonly poolAddress: EvmAddress;
  readonly token0: EvmAddress;
  readonly token1: EvmAddress;
  readonly coinIndex0: number;
  readonly coinIndex1: number;
  readonly balance0: bigint;
  readonly balance1: bigint;
  readonly liquidity: bigint;
};

type PairResult = {
  readonly pool: (Pool & { readonly kind: CurvePoolKind; }) | undefined;
  readonly failures: readonly DiscoveryFailure[];
};

export const curveDiscoverer = (options: CurveDiscovererOptions): Discoverer => {
  const metaRegistryAddress = canonicalizeAddress(options.metaRegistryAddress ?? CURVE_META_REGISTRY_MAINNET);
  const identity = options.identity ?? "curve";
  const minLiquidity = options.minLiquidity ?? 1n;

  return {
    identity: `${identity}:${metaRegistryAddress}`,
    discover: async ({ tokens, context }) => {
      const pairs = tokenPairs(tokens);
      const provider = context.getProvider(options.networkId);
      const decimals = await fetchDecimals(provider, tokens, context.blockNumber, options.concurrency ?? 16);
      const settled = await settleMap(
        pairs,
        options.concurrency ?? 16,
        async ([tokenA, tokenB]) => discoverPair(provider, metaRegistryAddress, tokenA, tokenB, {
          minLiquidity,
          probeAmount: pow10(decimalsOf(decimals, tokenA)),
          blockNumber: context.blockNumber,
        }),
      );
      const failures: DiscoveryFailure[] = [];
      const pools: Array<Pool & { readonly kind: CurvePoolKind; }> = [];
      let skipped = 0;

      for (const { input: [tokenA, tokenB], result } of settled) {
        if (result.status === "rejected") {
          failures.push({ target: `${tokenA}/${tokenB}`, message: failureMessage(result.reason), cause: result.reason });
          continue;
        }

        failures.push(...result.value.failures);

        if (result.value.pool === undefined) skipped += 1;
        else pools.push(result.value.pool);
      }

      const quoters = pools.map(pool => curveQuoter({
        networkId: options.networkId,
        poolAddress: pool.poolAddress,
        token0: pool.token0,
        token1: pool.token1,
        coinIndex0: pool.coinIndex0,
        coinIndex1: pool.coinIndex1,
        kind: pool.kind,
        confidence: confidence(pool, decimals),
        identityPrefix: identity,
      }));

      return { quoters, attempted: pairs.length, skipped, failures };
    },
  };
};

type PairDiscoveryOptions = {
  readonly minLiquidity: bigint;
  readonly probeAmount: bigint;
  readonly blockNumber: bigint | undefined;
};

// Bounds in-flight registry calls per pair; pairs already run concurrently.
const CANDIDATE_CONCURRENCY = 4;

// A pair can be served by many Curve pools; only the deepest one that answers
// a get_dy probe becomes a quoter, mirroring the per-pair dedup of V2/V3.
const discoverPair = async (
  provider: RpcProvider,
  metaRegistryAddress: EvmAddress,
  tokenA: EvmAddress,
  tokenB: EvmAddress,
  { minLiquidity, probeAmount, blockNumber }: PairDiscoveryOptions,
): Promise<PairResult> => {
  const poolsResult = await contractCall(provider, metaRegistryAddress, abi.findPoolsForCoins, [tokenA, tokenB], blockNumber);

  if (!isAddressArray(poolsResult)) throw new TypeError("Curve registry returned malformed pool list");

  const failures: DiscoveryFailure[] = [];
  const candidates: Pool[] = [];
  const settled = await settleMap(poolsResult.map(canonicalizeAddress), CANDIDATE_CONCURRENCY, async (poolAddress) => {
    const [indicesResult, balancesResult] = await Promise.all([
      contractCall(provider, metaRegistryAddress, abi.getCoinIndices, [poolAddress, tokenA, tokenB], blockNumber),
      contractCall(provider, metaRegistryAddress, abi.getBalances, [poolAddress], blockNumber),
    ]);

    return { indicesResult, balancesResult };
  });

  for (const { input: poolAddress, result } of settled) {
    if (result.status === "rejected") {
      failures.push({ target: poolAddress, message: failureMessage(result.reason), cause: result.reason });
      continue;
    }

    const indices = normalizeCoinIndices(result.value.indicesResult);
    const balances = normalizeBalances(result.value.balancesResult);

    if (indices === undefined || balances === undefined) {
      failures.push({ target: poolAddress, message: "Curve registry returned malformed pool data" });
      continue;
    }

    // Underlying-only matches (metapool base coins) need get_dy_underlying
    // and are skipped in favor of pools holding both coins directly.
    if (indices.isUnderlying) continue;

    const balance0 = balances[indices.coinIndex0] ?? 0n;
    const balance1 = balances[indices.coinIndex1] ?? 0n;
    const liquidity = minBigInt(balance0, balance1);

    if (liquidity < minLiquidity) continue;

    candidates.push({
      poolAddress,
      token0: tokenA,
      token1: tokenB,
      coinIndex0: indices.coinIndex0,
      coinIndex1: indices.coinIndex1,
      balance0,
      balance1,
      liquidity,
    });
  }

  candidates.sort((left, right) => {
    if (left.liquidity !== right.liquidity) return left.liquidity < right.liquidity ? 1 : -1;

    return left.poolAddress.toLowerCase() < right.poolAddress.toLowerCase() ? -1 : 1;
  });

  for (const candidate of candidates) {
    const kind = await probePoolKind(provider, candidate, probeAmount, blockNumber);

    if (kind !== undefined) return { pool: { ...candidate, kind }, failures };

    failures.push({ target: candidate.poolAddress, message: "get_dy probe failed for both index encodings" });
  }

  return { pool: undefined, failures };
};

// StableSwap and crypto pools share no get_dy selector, so one probe per
// encoding identifies which variant the pool speaks. The probe trades a whole
// unit of token0 because crypto-pool math reverts on dust-sized inputs.
const probePoolKind = async (
  provider: RpcProvider,
  pool: Pool,
  probeAmount: bigint,
  blockNumber: bigint | undefined,
): Promise<CurvePoolKind | undefined> => {
  const probeArgs = [BigInt(pool.coinIndex0), BigInt(pool.coinIndex1), probeAmount];

  try {
    await contractCall(provider, pool.poolAddress, abi.getDyStableSwap, probeArgs, blockNumber);

    return "stableswap";
  }
  catch {
    // Not a StableSwap pool; fall through to the crypto encoding.
  }

  try {
    await contractCall(provider, pool.poolAddress, abi.getDyCrypto, probeArgs, blockNumber);

    return "crypto";
  }
  catch {
    return undefined;
  }
};

const tokenPairs = (tokens: readonly EvmAddress[]): Array<readonly [EvmAddress, EvmAddress]> => {
  const pairs: Array<readonly [EvmAddress, EvmAddress]> = [];

  for (const [left, tokenA] of tokens.entries()) {
    for (const tokenB of tokens.slice(left + 1)) pairs.push([tokenA, tokenB]);
  }

  return pairs;
};

const confidence = (pool: Pool, decimals: ReadonlyMap<string, number>): number => {
  const units0 = Number(pool.balance0) / 10 ** decimalsOf(decimals, pool.token0);
  const units1 = Number(pool.balance1) / 10 ** decimalsOf(decimals, pool.token1);

  return Math.round(liquidityConfidence(Math.sqrt(units0 * units1)));
};

const minBigInt = (left: bigint, right: bigint): bigint => (left < right ? left : right);
const isAddress = (value: unknown): value is EvmAddress => typeof value === "string" && value.startsWith("0x");
const isAddressArray = (value: unknown): value is readonly EvmAddress[] => Array.isArray(value) && value.every(isAddress);

const normalizeCoinIndices = (value: unknown): { coinIndex0: number; coinIndex1: number; isUnderlying: boolean; } | undefined => {
  let fields: { i: unknown; j: unknown; isUnderlying: unknown; } | undefined;

  if (Array.isArray(value)) {
    fields = { i: value[0], j: value[1], isUnderlying: value[2] };
  }
  else if (typeof value === "object" && value !== null && "i" in value && "j" in value && "is_underlying" in value) {
    fields = { i: value.i, j: value.j, isUnderlying: value.is_underlying };
  }

  if (fields === undefined || typeof fields.isUnderlying !== "boolean") return undefined;

  const coinIndex0 = decodedUint(fields.i);
  const coinIndex1 = decodedUint(fields.j);

  if (coinIndex0 === undefined || coinIndex1 === undefined || coinIndex0 > 7 || coinIndex1 > 7) return undefined;

  return { coinIndex0, coinIndex1, isUnderlying: fields.isUnderlying };
};

const normalizeBalances = (value: unknown): readonly bigint[] | undefined =>
  (Array.isArray(value) && value.every(entry => typeof entry === "bigint") ? value : undefined);
