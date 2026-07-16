import assert from "node:assert/strict";
import test from "node:test";

import { implementationScore, parityScore } from "./score.mjs";

test("price score penalizes inaccurate and missing routes", () => {
  const comparisons = [
    { consensusUsd: 100, rustUsd: 99, rustErrorPercent: 1 },
    { consensusUsd: 100 },
    { rustUsd: 1 },
  ];

  assert.deepEqual(implementationScore(comparisons, "rustUsd", "rustErrorPercent"), {
    priceScore: 49.5,
    meanAbsolutePercentageError: 1,
    pricedRoutes: 1,
    expectedPricedRoutes: 2,
    routeCoveragePercent: 2 / 3 * 100,
  });
});

test("parity only compares successful routes", () => {
  assert.deepEqual(parityScore([
    { rustUsd: 1, typescriptUsd: 1, crossLanguageDifferencePercent: 0 },
    { rustUsd: 1, typescriptUsd: 2, crossLanguageDifferencePercent: 50 },
    { rustUsd: 1 },
  ]), { exactMatches: 1, comparedRoutes: 2 });
});
