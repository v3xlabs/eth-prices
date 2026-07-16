import { describe, expect, it } from "vitest";

import { createNetworkContext } from "../src/network.js";
import { fixedQuoter } from "../src/quoters/fixed/index.js";

const context = createNetworkContext({});

describe("fixedQuoter", () => {
  it("matches Rust scaling in both directions", async () => {
    const quoter = fixedQuoter({
      assetIn: "token:18",
      assetInDecimals: 18,
      assetOut: "token:6",
      assetOutDecimals: 6,
      fixedRate: 1_000_000n,
      fixedRateDecimals: 6,
    });

    await expect(quoter.quote({ amountIn: 1_000_000_000_000_000_000n, direction: "forward", context }))
      .resolves.toBe(1_000_000n);
    await expect(quoter.quote({ amountIn: 1_000_000n, direction: "reverse", context }))
      .resolves.toBe(1_000_000_000_000_000_000n);
  });

  it("rejects invalid inputs with stable errors", async () => {
    const quoter = fixedQuoter({
      assetIn: "A",
      assetInDecimals: 0,
      assetOut: "B",
      assetOutDecimals: 0,
      fixedRate: 0n,
      fixedRateDecimals: 0,
    });

    await expect(quoter.quote({ amountIn: -1n, direction: "forward", context })).rejects.toThrow("amountIn");
    await expect(quoter.quote({ amountIn: 1n, direction: "reverse", context })).rejects.toThrow("zero rate");
  });

  it("bounds decimal exponents to Rust's u8 range", async () => {
    const quoter = fixedQuoter({
      assetIn: "A",
      assetInDecimals: 256,
      assetOut: "B",
      assetOutDecimals: 0,
      fixedRate: 1n,
      fixedRateDecimals: 0,
    });

    await expect(quoter.quote({ amountIn: 1n, direction: "forward", context })).rejects.toThrow("between 0 and 255");
  });
});
