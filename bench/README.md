# Autorouter Evaluation

This private workspace package measures the Rust and TypeScript autorouters with
the same Mainnet assets, protocol settings, quote block, and quote amounts. It
compares their USDC outputs with live keyless prices from:

- CoinGecko
- CoinMarketCap
- Coinbase
- Kraken
- Binance
- CoinPaprika

CoinMarketCap uses an undocumented website endpoint and is best effort. Binance
quotes use USDT as the USD proxy. A failed reference source is recorded without
discarding the other references.

## Commands

From the repository root:

```sh
just eval
just eval-rust
just eval-ts
pnpm --dir bench test
```

`just eval` builds the TypeScript package, selects one Ethereum block, fetches
the references, runs both implementations sequentially, and writes a complete
JSON report to `target/evals/`. `target/evals/latest.json` points to the most
recent result.

The runners accept these environment variables:

- `RPC_URL`: Ethereum JSON-RPC endpoint.
- `EVAL_BLOCK`: decimal block number. The orchestrator selects one when omitted.
- `EVAL_ITERATIONS`: route-computation iterations, defaulting to `1000`.
- `EVAL_SOURCE_TIMEOUT_MS`: per-source timeout, defaulting to `10000`.
- `EVAL_CASES`: alternate manifest path for the Rust runner.

## Interpretation

Discovery and quote timings include live RPC latency. Route-computation timings
are local microbenchmarks and should be compared on the same machine and build.
The external APIs report current market prices, so accuracy comparisons must use
a recent block. They are evaluations, not stable CI assertions.

TypeScript discovery is pinned through its network context. The current Rust
`AutoRouter` API discovers against the provider's latest state, while resulting
quotes use the shared pinned block. Reports expose this as `discoveryBlockPinned`
instead of implying stronger reproducibility than the API provides.

`cases.json` defines one production-style autorouter graph with V2, V3, and
ERC-4626 discovery enabled together. WETH, WBTC, ENS, EURC, AAVE, MORPHO, and a
MetaMorpho USDC vault are each quoted once rather than benchmarked per protocol.
Raw quote amounts remain decimal integer strings in reports; the orchestrator
normalizes them using the configured token decimals.

The report's `priceScore` is an accuracy score out of 100 across assets with
external references. Each successful route contributes `100 - percentage error`;
a missing route contributes zero, so both coverage and price quality improve the
same score. `meanAbsolutePercentageError` and total route coverage are reported
separately for diagnosis.
