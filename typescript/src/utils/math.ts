import { EthPricesError } from "../error.js";

export const assertUnsignedInteger = (value: number, name: string): void => {
  if (!Number.isSafeInteger(value) || value < 0 || value > 255) {
    throw new EthPricesError("INVALID_INPUT", `${name} must be an integer between 0 and 255`);
  }
};

export const pow10 = (exp: number): bigint => {
  assertUnsignedInteger(exp, "decimal count");

  return 10n ** BigInt(exp);
};

export const mulDiv = (value: bigint, numerator: bigint, denominator: bigint): bigint => {
  if (denominator === 0n) throw new EthPricesError("RATE_UNAVAILABLE", "Cannot divide by a zero rate");

  return (value * numerator) / denominator;
};

// Maps a pool's geometric-mean liquidity, expressed in whole token units so it
// is comparable across tokens with different decimals, onto 0..100. Log-scaled
// so pools rank by order of magnitude; saturates at one million whole units.
export const liquidityConfidence = (geometricMeanLiquidity: number): number => {
  if (!Number.isFinite(geometricMeanLiquidity) || geometricMeanLiquidity <= 0) return 0;

  const points = Math.round((100 / 6) * Math.log10(1 + geometricMeanLiquidity));

  return Math.min(100, Math.max(0, points));
};

const FRESHNESS_HALF_LIFE_SECONDS = 86_400;

// A pool that has not traded recently carries a spot price nobody has been
// willing to arbitrage, so its validity decays with the age of the last trade.
export const freshnessMultiplier = (ageSeconds: number): number => {
  if (!Number.isFinite(ageSeconds) || ageSeconds <= 0) return 1;

  return 2 ** (-ageSeconds / FRESHNESS_HALF_LIFE_SECONDS);
};
