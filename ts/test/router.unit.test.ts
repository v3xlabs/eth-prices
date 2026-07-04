import { describe, expect, it } from "vitest";

import { createNetworkContext } from "../src/network.js";
import { fixedQuoter } from "../src/quoters/fixed/index.js";
import { createRouter } from "../src/router/index.js";

const context = createNetworkContext({});

const fixed = (assetIn: string, assetOut: string, fixedRate = 1n, confidence = 0) => fixedQuoter({
  assetIn,
  assetInDecimals: 0,
  assetOut,
  assetOutDecimals: 0,
  fixedRate,
  fixedRateDecimals: 0,
  confidence,
});

describe("Router", () => {
  it("finds and executes a multi-hop route", async () => {
    const router = createRouter([fixed("A", "B", 2n), fixed("B", "C", 3n)]);
    const route = router.compute("A", "C");

    expect(route.path).toHaveLength(2);
    await expect(router.quote("A", "C", { amountIn: 100n, context })).resolves.toBe(600n);
  });

  it("uses confidence-weighted costs instead of minimum hops", () => {
    const router = createRouter([
      fixed("A", "C", 1n, 0),
      fixed("A", "B", 1n, 100),
      fixed("B", "C", 1n, 100),
    ]);

    expect(router.compute("A", "C").path.map(step => step.quoter.assets)).toEqual([
      ["A", "B"],
      ["B", "C"],
    ]);
  });

  it("retains competing parallel quoters and selects the highest confidence", () => {
    const router = createRouter([fixed("A", "B", 1n, 10), fixed("A", "B", 2n, 90)]);

    expect(router.quoters()).toHaveLength(2);
    expect(router.compute("A", "B").path[0]?.quoter.confidence).toBe(90);
  });

  it("canonicalizes EVM addresses", () => {
    const lower = "0x00000000000000000000000000000000000000ab";
    const upper = "0x00000000000000000000000000000000000000AB";
    const router = createRouter([fixed(lower, "fiat:usd")]);

    expect(router.compute(upper, "fiat:usd").path).toHaveLength(1);
  });

  it("returns an empty same-asset route", () => {
    const router = createRouter([fixed("A", "B")]);

    expect(router.compute("A", "A").path).toEqual([]);
  });

  it("reports missing and disconnected assets", () => {
    const router = createRouter([fixed("A", "B"), fixed("C", "D")]);

    expect(() => router.compute("missing", "B")).toThrow("Asset not found: missing");
    expect(() => router.compute("A", "D")).toThrow("No route found from A to D");
  });

  it("adds batches atomically", () => {
    const router = createRouter();

    expect(() => router.addQuoters([fixed("A", "B"), fixed("C", "C")])).toThrow("connects an asset to itself");
    expect(router.quoters()).toHaveLength(0);
  });
});
