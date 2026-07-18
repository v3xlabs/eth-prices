import assert from "node:assert/strict";
import test from "node:test";

import {
  createPalette,
  formatDuration,
  formatNanoseconds,
  formatPercent,
  formatRoute,
  formatUsd,
  renderReport,
  renderTable,
  significanceOf,
  stripAnsi,
} from "./report.mjs";
import { assessPrice, referenceStatistics } from "./score.mjs";

test("routes render as compact hops with pool prefixes", () => {
  assert.equal(formatRoute(["uniswap_v2:0xcbcdf2624d2b6e359265ac2f6a67e04b9c9ffe8a"]), "v2:cbcd");
  assert.equal(
    formatRoute(["erc4626:0x83f20f44975d03b1b09e64809b757c47f942beea", "uniswap_v3:0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"]),
    "4626:83f2→v3:88e6",
  );
  assert.equal(formatRoute(["ecb:fiat:eur:fiat:usd"]), "ecb");
  assert.equal(formatRoute([]), "—");
});

test("usd formatting adapts precision to magnitude", () => {
  assert.equal(formatUsd(undefined), "—");
  assert.equal(formatUsd(118_252.129), "118,252.13");
  assert.equal(formatUsd(3.14159), "3.1416");
  assert.equal(formatUsd(0.0123456), "0.01235");
  assert.equal(formatUsd(1e-9), "1.00e-9");
});

test("percent and duration formatting", () => {
  assert.equal(formatPercent(undefined), "—");
  assert.equal(formatPercent(0), "0.000%");
  assert.equal(formatPercent(0.0001), "<0.001%");
  assert.equal(formatPercent(1.23456), "1.235%");
  assert.equal(formatDuration(12_345), "12.3s");
  assert.equal(formatDuration(234.4), "234ms");
  assert.equal(formatDuration(0.5), "500µs");
  assert.equal(formatNanoseconds(850), "850ns");
  assert.equal(formatNanoseconds(2_500), "2.50µs");
});

test("significance classifies against the reference envelope", () => {
  const statistics = referenceStatistics([
    { source: "coinbase", priceUsd: 99 },
    { source: "kraken", priceUsd: 101 },
  ]);
  const implementation = price => ({ priceUsd: price, assessment: assessPrice(price, statistics) });

  assert.equal(significanceOf(implementation(100), statistics), "within");
  assert.equal(significanceOf(implementation(102.5), statistics), "near");
  assert.equal(significanceOf(implementation(110), statistics), "far");
  assert.equal(significanceOf({ error: "no route" }, statistics), "error");
  assert.equal(significanceOf({ priceUsd: 1 }, undefined), "unreferenced");
  assert.equal(significanceOf(undefined, statistics), "missing");
});

test("tables align columns ignoring ansi codes", () => {
  const palette = createPalette(true);
  const lines = renderTable(
    [{ label: "name" }, { label: "value", align: "right" }],
    [
      [palette.green("weth"), "3,772.51"],
      ["rail", palette.red("0.0123")],
    ],
    palette,
  );

  assert.equal(stripAnsi(lines[0]), "  name     value");
  assert.equal(stripAnsi(lines[2]), "  weth  3,772.51");
  assert.equal(stripAnsi(lines[3]), "  rail    0.0123");
});

test("renderReport produces a full plain-text report", () => {
  const statistics = referenceStatistics([
    { source: "coinbase", priceUsd: 100 },
    { source: "kraken", priceUsd: 102 },
  ]);
  const report = {
    blockNumber: 23_000_000,
    rpcUrl: "https://ethereum.reth.rs/rpc",
    iterations: 1000,
    implementationsRun: ["rust"],
    reportPath: "/tmp/report.json",
    references: {
      weth: { records: statistics.sources.map(source => ({ source: source.source, priceUsd: source.priceUsd })), errors: [] },
      wbtc: { records: [], errors: [{ source: "coinbase", error: "HTTP 500" }] },
    },
    implementations: {
      rust: {
        runs: [{
          name: "autorouter",
          discoveryMs: 1200,
          discoveredQuoters: 40,
          totalRpcRequests: 300,
          quotes: [{ asset: "weth", quoteMs: 100, routeComputeNs: 900 }],
        }],
      },
    },
    scores: {
      rust: {
        priceScore: 99.5,
        meanAbsolutePercentageError: 0.5,
        medianAbsolutePercentageError: 0.5,
        withinEnvelopeCount: 1,
        assessedCount: 2,
        pricedRoutes: 2,
        expectedPricedRoutes: 2,
        routeCoveragePercent: 100,
        worstCase: { case: "autorouter:wbtc", errorPercent: 5 },
      },
    },
    comparisons: [
      {
        case: "autorouter:weth",
        asset: "weth",
        reference: statistics,
        implementations: {
          rust: {
            priceUsd: 101,
            assessment: assessPrice(101, statistics),
            sources: ["uniswap_v3:0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"],
            quoteMs: 100,
          },
        },
      },
      {
        case: "autorouter:rail",
        asset: "rail",
        reference: { count: 0, sources: [] },
        implementations: { rust: { error: "no route found" } },
      },
    ],
    baseline: {
      generatedAt: "2026-07-17T00:00:00.000Z",
      blockNumber: 22_999_000,
      scores: [{
        implementation: "rust",
        priceScore: { from: 98, to: 99.5 },
        meanAbsolutePercentageError: { from: 1.2, to: 0.5 },
        routeCoveragePercent: { from: 100, to: 100 },
      }],
      caseDeltas: [{ case: "autorouter:weth", implementation: "rust", from: 1.2, to: 0.5 }],
    },
  };

  const text = renderReport(report, { colors: false });
  assert.match(text, /block 23000000/);
  assert.match(text, /coinbase\s+✓ 1 ✗ 1 \(wbtc: HTTP 500\)/);
  assert.match(text, /rust\s+99\.50/);
  assert.match(text, /worst rust: autorouter:wbtc at 5\.000%/);
  assert.match(text, /weth\s+101\.0000/);
  assert.match(text, /v3:88e6/);
  assert.match(text, /✗ rust rail: no route found/);
  assert.match(text, /vs baseline/);
  assert.match(text, /1\.200% → 0\.500%/);
  assert.doesNotMatch(text, /\u001B/);
});
