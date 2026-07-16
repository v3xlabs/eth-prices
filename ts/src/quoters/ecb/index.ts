import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { mulDiv, pow10 } from "../../utils/math.js";

const RATE_DECIMALS = 6;
const RATE_SCALE = pow10(RATE_DECIMALS);
const ECB_BASE_URL = "https://data-api.ecb.europa.eu/service/data/EXR/D..EUR.SP00.A";

export const ECB_CURRENCIES = [
  "ars", "aud", "bgn", "brl", "cad", "chf", "cny", "cyp", "czk", "dkk", "dzd", "eek", "gbp",
  "grd", "hkd", "hrk", "huf", "idr", "ils", "inr", "isk", "jpy", "krw", "ltl", "lvl", "mad",
  "mtl", "mxn", "myr", "nok", "nzd", "php", "pln", "ron", "rub", "sek", "sgd", "sit", "skk",
  "thb", "try", "twd", "usd", "zar",
];

export type EcbRateSource = {
  rateFor(symbol: string, timestamp: bigint): Promise<bigint>;
  quoters(): readonly Quoter[];
};

export type EcbRateSourceOptions = {
  readonly timeoutMs?: number;
  readonly maxCachedDates?: number;
  readonly fetch?: typeof globalThis.fetch;
};

export type EcbQuoterParams = {
  readonly quoteSymbol: string;
  readonly rateSource: EcbRateSource;
  readonly confidence?: number;
};

export const ecbQuoter = (params: EcbQuoterParams): Quoter => {
  const quoteSymbol = params.quoteSymbol.toLowerCase();

  return {
    identity: `ecb:fiat:eur:fiat:${quoteSymbol}`,
    assets: ["fiat:eur", `fiat:${quoteSymbol}`],
    confidence: params.confidence ?? 0,
    quote: async ({ amountIn, direction, context }: QuoteParams) => {
      if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

      if (context.fiatTimestamp === undefined) {
        throw new EthPricesError("INVALID_INPUT", "ECB quoter requires a fiat timestamp");
      }

      const rate = await params.rateSource.rateFor(quoteSymbol, context.fiatTimestamp);

      return direction === "forward" ? mulDiv(amountIn, rate, RATE_SCALE) : mulDiv(amountIn, RATE_SCALE, rate);
    },
  };
};

export const createEcbRateSource = (options: EcbRateSourceOptions = {}): EcbRateSource => {
  const fetcher = options.fetch ?? globalThis.fetch;
  const timeoutMs = options.timeoutMs ?? 10_000;
  const maxCachedDates = options.maxCachedDates ?? 32;
  const cache = new Map<string, Map<string, bigint>>();
  const inFlight = new Map<string, Promise<Map<string, bigint>>>();

  if (fetcher === undefined) throw new EthPricesError("INVALID_CONFIGURATION", "A fetch implementation is required");

  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
    throw new EthPricesError("INVALID_CONFIGURATION", "timeoutMs must be a positive safe integer");
  }

  if (!Number.isSafeInteger(maxCachedDates) || maxCachedDates <= 0) {
    throw new EthPricesError("INVALID_CONFIGURATION", "maxCachedDates must be a positive safe integer");
  }

  const loadRates = async (date: string): Promise<Map<string, bigint>> => {
    const cached = cache.get(date);

    if (cached !== undefined) return cached;

    const pending = inFlight.get(date);

    if (pending !== undefined) return pending;

    const request = fetchRates(date, fetcher, timeoutMs)
      .then((rates) => {
        cache.set(date, rates);
        while (cache.size > maxCachedDates) {
          const oldest = cache.keys().next().value;

          if (typeof oldest !== "string") break;

          cache.delete(oldest);
        }

        return rates;
      })
      .finally(() => inFlight.delete(date));

    inFlight.set(date, request);

    return request;
  };

  const source: EcbRateSource = {
    rateFor: async (symbol, timestamp) => {
      if (timestamp < 0n) throw new EthPricesError("INVALID_INPUT", "timestamp must not be negative");

      const date = unixTimestampToDate(timestamp);
      const rates = await loadRates(date);
      const rate = rates.get(symbol.toLowerCase());

      if (rate === undefined) throw new EthPricesError("RATE_UNAVAILABLE", `ECB rate unavailable for fiat:${symbol}`);

      return rate;
    },
    quoters: () => ECB_CURRENCIES.map(quoteSymbol => ecbQuoter({ quoteSymbol, rateSource: source })),
  };

  return source;
};

const fetchRates = async (date: string, fetcher: typeof globalThis.fetch, timeoutMs: number): Promise<Map<string, bigint>> => {
  const url = `${ECB_BASE_URL}?endPeriod=${date}&lastNObservations=1&format=csvdata`;
  const response = await fetcher(url, { signal: AbortSignal.timeout(timeoutMs) });

  if (!response.ok) throw new EthPricesError("RATE_UNAVAILABLE", `ECB API returned ${response.status}`);

  return parseRates(await response.text());
};

const parseRates = (body: string): Map<string, bigint> => {
  const lines = body.trim().split(/\r?\n/u);
  const headers = parseCsvLine(lines[0] ?? "");
  const currencyIndex = headers.indexOf("CURRENCY");
  const valueIndex = headers.indexOf("OBS_VALUE");

  if (currencyIndex === -1 || valueIndex === -1) throw new EthPricesError("RATE_UNAVAILABLE", "ECB CSV is missing required columns");

  const rates = new Map<string, bigint>();

  for (const line of lines.slice(1)) {
    const columns = parseCsvLine(line);
    const symbol = columns[currencyIndex];
    const value = columns[valueIndex];

    if (symbol === undefined || value === undefined) throw new EthPricesError("RATE_UNAVAILABLE", "ECB CSV contains a malformed row");

    rates.set(symbol.toLowerCase(), parseDecimalRate(value));
  }

  return rates;
};

const parseCsvLine = (line: string): string[] => {
  const columns: string[] = [];
  let value = "";
  let quoted = false;

  for (let index = 0; index < line.length; index++) {
    const character = line[index];

    if (character === "\"") {
      if (quoted && line[index + 1] === "\"") {
        value += "\"";
        index += 1;
      }
      else quoted = !quoted;
    }
    else if (character === "," && !quoted) {
      columns.push(value);
      value = "";
    }
    else value += character;
  }
  columns.push(value);

  return columns;
};

const parseDecimalRate = (value: string): bigint => {
  if (!/^\d+(?:\.\d+)?$/u.test(value)) throw new EthPricesError("RATE_UNAVAILABLE", `Invalid ECB rate: ${value}`);

  const [integer, fractional = ""] = value.split(".");

  return BigInt(`${integer}${fractional.padEnd(RATE_DECIMALS, "0").slice(0, RATE_DECIMALS)}`);
};

const unixTimestampToDate = (timestamp: bigint): string => new Date(Number(timestamp * 1000n)).toISOString()
.slice(0, 10);
