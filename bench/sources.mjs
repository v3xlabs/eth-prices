const SOURCE_NAMES = {
  binance: "binance",
  coinbase: "coinbase",
  coingecko: "coingecko",
  coinmarketcap: "coinmarketcap",
  coinpaprika: "coinpaprika",
  kraken: "kraken",
};

function positivePrice(value, source) {
  const price = typeof value === "string" && value.trim() !== "" ? Number(value) : value;
  if (typeof price !== "number" || !Number.isFinite(price) || price <= 0) {
    throw new Error(`${source} returned an invalid USD price`);
  }
  return price;
}

function isoDate(value, source) {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) {
    throw new Error(`${source} returned an invalid observation time`);
  }
  return date.toISOString();
}

export function parseCoinGecko(payload, coinGeckoId) {
  const quote = payload?.[coinGeckoId];
  const record = { priceUsd: positivePrice(quote?.usd, SOURCE_NAMES.coingecko) };
  if (quote?.last_updated_at !== undefined) {
    record.observedAt = isoDate(quote.last_updated_at * 1_000, SOURCE_NAMES.coingecko);
  }
  return record;
}

export function parseCoinMarketCap(payload) {
  const market = payload?.data?.marketPairs?.[0];
  if (market) {
    const record = { priceUsd: positivePrice(market.price, SOURCE_NAMES.coinmarketcap) };
    if (market.lastUpdated !== undefined) {
      record.observedAt = isoDate(market.lastUpdated, SOURCE_NAMES.coinmarketcap);
    }
    return record;
  }

  const points = payload?.data?.points;
  if (points && typeof points === "object") {
    const latest = Object.entries(points)
      .filter(([timestamp, point]) => Number.isFinite(Number(timestamp)) && Array.isArray(point?.v))
      .sort(([left], [right]) => Number(right) - Number(left))[0];
    if (latest) {
      return {
        priceUsd: positivePrice(latest[1].v[0], SOURCE_NAMES.coinmarketcap),
        observedAt: isoDate(Number(latest[0]) * 1_000, SOURCE_NAMES.coinmarketcap),
      };
    }
  }

  const data = payload?.data;
  const price = data?.quote?.USD?.price ?? data?.statistics?.price ?? data?.price;
  const record = { priceUsd: positivePrice(price, SOURCE_NAMES.coinmarketcap) };
  const observedAt = data?.quote?.USD?.last_updated ?? data?.last_updated ?? data?.lastUpdated;
  if (observedAt !== undefined) {
    record.observedAt = isoDate(observedAt, SOURCE_NAMES.coinmarketcap);
  }
  return record;
}

export function parseCoinbase(payload) {
  return { priceUsd: positivePrice(payload?.data?.amount, SOURCE_NAMES.coinbase) };
}

export function parseKraken(payload) {
  if (Array.isArray(payload?.error) && payload.error.length > 0) {
    throw new Error(`kraken returned an error: ${payload.error.join(", ")}`);
  }
  const ticker = payload?.result && Object.values(payload.result)[0];
  return { priceUsd: positivePrice(ticker?.c?.[0], SOURCE_NAMES.kraken) };
}

export function parseBinance(payload) {
  return { priceUsd: positivePrice(payload?.price, SOURCE_NAMES.binance) };
}

export function parseCoinPaprika(payload) {
  const record = {
    priceUsd: positivePrice(payload?.quotes?.USD?.price, SOURCE_NAMES.coinpaprika),
  };
  if (payload?.last_updated !== undefined) {
    record.observedAt = isoDate(payload.last_updated, SOURCE_NAMES.coinpaprika);
  }
  return record;
}

const sourceDefinitions = [
  {
    source: SOURCE_NAMES.coingecko,
    key: "coinGeckoId",
    url: (value) => `https://api.coingecko.com/api/v3/simple/price?ids=${encodeURIComponent(value)}&vs_currencies=usd&include_last_updated_at=true`,
    parse: parseCoinGecko,
  },
  {
    source: SOURCE_NAMES.coinmarketcap,
    key: "coinMarketCapId",
    url: (value) => `https://api.coinmarketcap.com/data-api/v3/cryptocurrency/market-pairs/latest?id=${encodeURIComponent(value)}&start=1&limit=1&category=spot&centerType=all&sort=cmc_rank_advanced`,
    parse: parseCoinMarketCap,
  },
  {
    source: SOURCE_NAMES.coinbase,
    key: "coinbasePair",
    url: (value) => `https://api.coinbase.com/v2/prices/${encodeURIComponent(value)}/spot`,
    parse: parseCoinbase,
  },
  {
    source: SOURCE_NAMES.kraken,
    key: "krakenPair",
    url: (value) => `https://api.kraken.com/0/public/Ticker?pair=${encodeURIComponent(value)}`,
    parse: parseKraken,
  },
  {
    source: SOURCE_NAMES.binance,
    key: "binanceSymbol",
    url: (value) => `https://api.binance.com/api/v3/ticker/price?symbol=${encodeURIComponent(value)}`,
    parse: parseBinance,
  },
  {
    source: SOURCE_NAMES.coinpaprika,
    key: "coinPaprikaId",
    url: (value) => `https://api.coinpaprika.com/v1/tickers/${encodeURIComponent(value)}`,
    parse: parseCoinPaprika,
  },
];

async function fetchSource(definition, value, fetchImplementation, timeoutMs) {
  const controller = new AbortController();
  let timeout;
  const timeoutFailure = new Promise((_, reject) => {
    timeout = setTimeout(() => {
      controller.abort();
      reject(new Error(`timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  try {
    const response = await Promise.race([
      fetchImplementation(definition.url(value), {
        headers: { accept: "application/json" },
        signal: controller.signal,
      }),
      timeoutFailure,
    ]);
    if (!response?.ok) {
      throw new Error(`HTTP ${response?.status ?? "error"}`);
    }
    const payload = await Promise.race([response.json(), timeoutFailure]);
    return {
      source: definition.source,
      ...definition.parse(payload, value),
      fetchedAt: new Date().toISOString(),
    };
  } finally {
    clearTimeout(timeout);
  }
}

export async function fetchUsdSources(asset, options = {}) {
  const fetchImplementation = options.fetch ?? globalThis.fetch;
  const timeoutMs = options.timeoutMs ?? 5_000;
  if (typeof fetchImplementation !== "function") {
    throw new Error("fetch must be a function");
  }
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error("timeoutMs must be a finite positive number");
  }

  const supported = sourceDefinitions.filter(({ key }) =>
    typeof asset?.[key] === "string" && asset[key].trim() !== ""
  );
  const settled = await Promise.allSettled(
    supported.map((definition) =>
      fetchSource(definition, asset[definition.key], fetchImplementation, timeoutMs)
    )
  );
  const records = [];
  const errors = [];

  settled.forEach((result, index) => {
    if (result.status === "fulfilled") {
      records.push(result.value);
    } else {
      errors.push({
        source: supported[index].source,
        error: result.reason instanceof Error ? result.reason.message : String(result.reason),
      });
    }
  });

  return { records, errors };
}

export const fetchEthUsdSources = fetchUsdSources;
