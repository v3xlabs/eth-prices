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
just eval                       # both implementations, live references
just eval --impl rust           # only the Rust autorouter
just eval --impl ts --refs latest   # fast loop: reuse last references + block
just eval-rust                  # raw Rust runner JSON on stdout
just eval-ts                    # raw TypeScript runner JSON on stdout
pnpm --dir bench test
```

`node bench/run.mjs --help` lists every flag:

- `--impl rust|ts|both`: implementations to evaluate. Default `both`.
- `--refs latest|<path>`: reuse the references *and* block number of a previous
  report instead of refetching the external APIs. Reports are self-contained
  snapshots, so any file under `target/evals/` works.
- `--baseline latest|<path>`: report to diff against. Defaults to
  `target/evals/latest.json` (read before it is overwritten).
- `--block <n>` / `--iterations <n>`: pin the block, set route iterations.
- `--json`: print the full report JSON to stdout instead of the pretty output.
- `--no-color`: disable ANSI colors (also honors `NO_COLOR`; non-TTY output is
  plain unless `FORCE_COLOR` is set).

Environment variables (`RPC_URL`, `EVAL_BLOCK`, `EVAL_ITERATIONS`,
`EVAL_SOURCE_TIMEOUT_MS`, `EVAL_SKIP_TS_BUILD`, `EVAL_RUST_BINARY`,
`EVAL_CASES`) still work; flags take precedence.

Every run writes a timestamped JSON report to `target/evals/` and updates
`target/evals/latest.json`.

## Iterating on the autorouter

The intended loop for a human or agent improving either implementation:

1. `just eval` once to establish references and a baseline report.
2. Edit the router (Rust in `pkg/`, TypeScript in `ts/`).
3. `just eval --impl rust --refs latest` (or `--impl ts`). This skips the
   external price APIs, pins the same block, and auto-diffs against the
   previous report, so any score movement is caused by your change alone.
4. Read the `vs baseline` section, or parse `target/evals/latest.json`
   (`--json` prints the same document to stdout). Per-case data lives in
   `comparisons[]`: each entry carries the reference statistics, both
   implementations' prices, assessments, route (`sources`), timings, and full
   error strings.

Runner internals: `run.mjs` orchestrates, `bench/src/main.rs` and
`bench/typescript.mjs` emit one JSON document each with identical shapes,
including per-discoverer reports (attempted/discovered/skipped/failures) and
RPC request counts.

## Interpretation

Each quoted asset is compared against **all** of its reference records, not a
single number:

- **consensus** is the median reference price; `err` is the deviation from it.
- **±spread** is `(max − min) / median` across the references — how much the
  references themselves disagree. An implementation error smaller than the
  spread is not meaningful.
- The **envelope** is `[min, max]` of the references. `●` means the price is
  inside it (the error is insignificant), `◐` outside by at most the spread,
  `○` beyond that — a real deviation worth investigating.
- `deviationSigmas` in the JSON is the deviation measured in units of the
  references' median absolute deviation (only for ≥3 disagreeing sources);
  `nearestSource` names the reference the implementation tracks closest.

The `priceScore` is an accuracy score out of 100 across assets with external
references. Each successful route contributes `100 - percentage error`; a
missing route contributes zero, so both coverage and price quality improve the
same score. `meanAbsolutePercentageError`, the median error, the in-band count,
and route coverage are reported separately for diagnosis. `parity` compares the
two implementations directly and lists any diverging cases.

Discovery and quote timings include live RPC latency. Route-computation timings
are local microbenchmarks and should be compared on the same machine and build.
The external APIs report current market prices, so accuracy comparisons must
use a recent block (a snapshot via `--refs` keeps a whole iteration session
coherent). They are evaluations, not stable CI assertions.

TypeScript discovery is pinned through its network context. The current Rust
`AutoRouter` API discovers against the provider's latest state, while resulting
quotes use the shared pinned block. Reports expose this as `discoveryBlockPinned`
instead of implying stronger reproducibility than the API provides.

`cases.json` defines one production-style autorouter graph with V2, V3, and
ERC-4626 discovery enabled together. Raw quote amounts remain decimal integer
strings in reports; the orchestrator normalizes them using the configured token
decimals.
