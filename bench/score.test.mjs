import assert from "node:assert/strict";
import test from "node:test";

import { assessPrice, implementationScore, parityScore, referenceStatistics } from "./score.mjs";

const record = (source, priceUsd) => ({ source, priceUsd });

test("reference statistics summarize spread and per-source deviation", () => {
  const statistics = referenceStatistics([
    record("coinbase", 99),
    record("kraken", 100),
    record("binance", 102),
  ]);

  assert.equal(statistics.count, 3);
  assert.equal(statistics.min, 99);
  assert.equal(statistics.max, 102);
  assert.equal(statistics.median, 100);
  assert.equal(statistics.spreadPercent, 3);
  assert.equal(statistics.medianAbsoluteDeviation, 1);
  assert.equal(statistics.madPercent, 1);
  assert.deepEqual(statistics.sources.map(source => source.deviationPercent), [-1, 0, 2]);
});

test("reference statistics handle empty and single-source references", () => {
  assert.deepEqual(referenceStatistics([]), { count: 0, sources: [] });

  const single = referenceStatistics([record("coinbase", 50)]);
  assert.equal(single.count, 1);
  assert.equal(single.median, 50);
  assert.equal(single.spreadPercent, 0);
  assert.equal(single.madPercent, 0);
});

test("assessing a price inside the envelope reports zero envelope distance", () => {
  const statistics = referenceStatistics([
    record("coinbase", 99),
    record("kraken", 100),
    record("binance", 102),
  ]);
  const assessment = assessPrice(101, statistics);

  assert.equal(assessment.errorPercent, 1);
  assert.equal(assessment.withinEnvelope, true);
  assert.equal(assessment.envelopeDistancePercent, 0);
  assert.equal(assessment.deviationSigmas, 1);
  assert.equal(assessment.nearestSource, "kraken");
});

test("assessing a price outside the envelope reports the distance beyond it", () => {
  const statistics = referenceStatistics([
    record("coinbase", 99),
    record("kraken", 100),
    record("binance", 102),
  ]);
  const assessment = assessPrice(105, statistics);

  assert.equal(assessment.errorPercent, 5);
  assert.equal(assessment.withinEnvelope, false);
  assert.equal(assessment.envelopeDistancePercent, 3);
  assert.equal(assessment.deviationSigmas, 5);
  assert.equal(assessment.nearestSource, "binance");
});

test("assessment omits sigmas for thin or fully agreeing references", () => {
  const two = assessPrice(1.5, referenceStatistics([record("coinbase", 1), record("kraken", 2)]));
  assert.equal(two.deviationSigmas, undefined);

  const agreeing = assessPrice(1.02, referenceStatistics([
    record("coinbase", 1),
    record("kraken", 1),
    record("binance", 1),
  ]));
  assert.equal(agreeing.deviationSigmas, undefined);
  assert.equal(agreeing.withinEnvelope, false);
  assert.equal(agreeing.envelopeDistancePercent.toFixed(6), "2.000000");
});

test("assessment is undefined without a price or references", () => {
  const statistics = referenceStatistics([record("coinbase", 1)]);
  assert.equal(assessPrice(undefined, statistics), undefined);
  assert.equal(assessPrice(1, undefined), undefined);
  assert.equal(assessPrice(1, referenceStatistics([])), undefined);
});

function comparison(name, reference, implementations) {
  return { case: `autorouter:${name}`, asset: name, reference, implementations };
}

test("implementation score penalizes inaccurate and missing routes", () => {
  const reference = referenceStatistics([record("coinbase", 100)]);
  const comparisons = [
    comparison("weth", reference, { rust: { priceUsd: 99, assessment: assessPrice(99, reference) } }),
    comparison("wbtc", reference, { rust: { error: "no route" } }),
    comparison("usdt", undefined, { rust: { priceUsd: 1 } }),
  ];
  const score = implementationScore(comparisons, "rust");

  assert.equal(score.priceScore, 49.5);
  assert.equal(score.meanAbsolutePercentageError, 1);
  assert.equal(score.medianAbsolutePercentageError, 1);
  assert.equal(score.pricedRoutes, 1);
  assert.equal(score.expectedPricedRoutes, 2);
  assert.equal(score.routeCoveragePercent, 2 / 3 * 100);
  assert.equal(score.withinEnvelopeCount, 0);
  assert.equal(score.assessedCount, 1);
  assert.deepEqual(score.worstCase, { case: "autorouter:weth", errorPercent: 1 });
});

test("implementation score counts prices inside the reference envelope", () => {
  const reference = referenceStatistics([record("coinbase", 99), record("kraken", 101)]);
  const comparisons = [
    comparison("weth", reference, { typescript: { priceUsd: 100, assessment: assessPrice(100, reference) } }),
    comparison("wbtc", reference, { typescript: { priceUsd: 110, assessment: assessPrice(110, reference) } }),
  ];
  const score = implementationScore(comparisons, "typescript");

  assert.equal(score.withinEnvelopeCount, 1);
  assert.equal(score.assessedCount, 2);
  assert.equal(score.worstCase.case, "autorouter:wbtc");
});

test("parity compares only cases priced by both implementations", () => {
  const score = parityScore([
    comparison("weth", undefined, { rust: { priceUsd: 1 }, typescript: { priceUsd: 1 } }),
    comparison("wbtc", undefined, { rust: { priceUsd: 1 }, typescript: { priceUsd: 2 } }),
    comparison("dai", undefined, { rust: { priceUsd: 1 }, typescript: {} }),
  ]);

  assert.equal(score.exactMatches, 1);
  assert.equal(score.comparedRoutes, 2);
  assert.equal(score.maxDifferencePercent, 50);
  assert.equal(score.meanDifferencePercent, 25);
  assert.deepEqual(score.divergentCases, [{ case: "autorouter:wbtc", differencePercent: 50 }]);
});

test("parity handles single-implementation runs", () => {
  const score = parityScore([comparison("weth", undefined, { rust: { priceUsd: 1 } })]);
  assert.equal(score.comparedRoutes, 0);
  assert.equal(score.maxDifferencePercent, undefined);
  assert.deepEqual(score.divergentCases, []);
});
