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
