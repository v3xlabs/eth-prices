import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  fetchEthUsdSources,
  parseBinance,
  parseCoinbase,
  parseCoinGecko,
  parseCoinMarketCap,
  parseCoinPaprika,
  parseKraken,
} from "./sources.mjs";

async function fixture(name) {
  const contents = await readFile(new URL(`./fixtures/${name}.json`, import.meta.url), "utf8");
  return JSON.parse(contents);
}

test("source parsers normalize all six fixture formats", async () => {
  const [coingecko, coinmarketcap, coinbase, kraken, binance, coinpaprika] =
    await Promise.all([
      fixture("coingecko"),
      fixture("coinmarketcap"),
      fixture("coinbase"),
      fixture("kraken"),
      fixture("binance"),
      fixture("coinpaprika"),
    ]);

  assert.deepEqual(parseCoinGecko(coingecko, "ethereum"), {
    priceUsd: 3210.45,
    observedAt: "2024-03-09T16:00:00.000Z",
  });
  assert.deepEqual(parseCoinMarketCap(coinmarketcap), {
    priceUsd: 3201.25,
    observedAt: "2024-03-09T16:01:00.000Z",
  });
  assert.deepEqual(parseCoinbase(coinbase), { priceUsd: 3198.75 });
  assert.deepEqual(parseKraken(kraken), { priceUsd: 3202.4 });
  assert.deepEqual(parseBinance(binance), { priceUsd: 3203.56 });
  assert.deepEqual(parseCoinPaprika(coinpaprika), {
    priceUsd: 3204.67,
    observedAt: "2024-03-09T16:01:00.000Z",
  });
});

test("parsers reject non-positive and non-finite prices", () => {
  for (const invalid of [0, -1, "not-a-number", "Infinity"]) {
    assert.throws(() => parseBinance({ price: invalid }), /invalid USD price/);
  }
});

test("fetches supported sources concurrently and isolates source errors", async () => {
  const responses = new Map([
    ["api.coingecko.com", await fixture("coingecko")],
    ["api.coinbase.com", await fixture("coinbase")],
    ["api.kraken.com", await fixture("kraken")],
  ]);
  let pending = 0;
  let maximumPending = 0;
  const fetch = async (url) => {
    pending += 1;
    maximumPending = Math.max(maximumPending, pending);
    await new Promise((resolve) => setTimeout(resolve, 5));
    pending -= 1;
    const host = new URL(url).host;
    if (host === "api.kraken.com") {
      return { ok: false, status: 503, json: async () => ({}) };
    }
    return { ok: true, status: 200, json: async () => responses.get(host) };
  };

  const result = await fetchEthUsdSources(
    {
      coinGeckoId: "ethereum",
      coinbasePair: "ETH-USD",
      krakenPair: "ETHUSD",
    },
    { fetch, timeoutMs: 100 }
  );

  assert.equal(maximumPending, 3);
  assert.deepEqual(result.records.map(({ source, priceUsd }) => ({ source, priceUsd })), [
    { source: "coingecko", priceUsd: 3210.45 },
    { source: "coinbase", priceUsd: 3198.75 },
  ]);
  assert.match(result.records[0].fetchedAt, /^\d{4}-\d{2}-\d{2}T/);
  assert.deepEqual(result.errors, [{ source: "kraken", error: "HTTP 503" }]);
});

test("reports a timeout when an injected fetch does not settle", async () => {
  const result = await fetchEthUsdSources(
    { binanceSymbol: "ETHUSDT" },
    { fetch: async () => new Promise(() => {}), timeoutMs: 5 }
  );

  assert.deepEqual(result, {
    records: [],
    errors: [{ source: "binance", error: "timed out after 5ms" }],
  });
});
