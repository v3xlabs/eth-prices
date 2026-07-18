<h1 align="center">
    eth-prices
</h1>

<p align="center">
  A smol rust crate for fetching Ethereum asset prices directly from rpc.
</p>
<p align="center">
    <a href="https://docs.rs/eth-prices"><img src="https://img.shields.io/badge/Docs.rs-blue?logo=rust&color=brown&style=flat" alt="Documentation"></a>
    <a href="https://crates.io/crates/eth-prices"><img src="https://img.shields.io/badge/Crate.io-yellow?logo=data:image/x-icon;base64%2CiVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAMAAAAoLQ9TAAACylBMVEUAAADqvWfotVLot1n/wi3lt1/grEfls1HnuFzXnCnXnS3luWTfzavgq0PlwXvltFXlrkTnt4KJYzl4XDz/yHwUEAsIBgQXEgyhek+DZ0j///9JOShFNibsxHnls1Llsk7ntVTpu2Lnv3Lntlf///bhrkrkslHnumTouFvpuV3otlbntFDotlbZoDDpuV7ot1fntlbnumLWnjLirEHlsUvcpz/lr0XlqznmsEjsvGDaozbmrDrgq0LpyIbmtlnlslDntVbdqUXmrTzfq0TbqUXcp0DirELgrUjLlS3OlCPiqTjkrULDjCLDiRfbozfrs0WUahe4fw3hqTrhrEVYRCiQaCfDlk2yjV0yJxppUDZ1WzqEYCKMZzqZd1GxjmgAAAALCAYEAwIwJRt2WjmKZi6fbxeifVKKZ0J3VjNlTC8AAAAAAAALCAVCMiI0KB1DMx+NajaIZDp5WDSFZkOEak4AAAAcFQ5xVzyffViAYDxkTjXnu2XnuV/dpzzjtFnlsk/ntFPntlnnu2Tou2Tbozfjrkfntlfou2XoumLntFLntlbnumLnu2Pir0znvWznumPntljotFDnskrkqzzmrkDbojTgrEfgqkHmr0XiqTvgpzbmrT3mrDjnrDjQliLTmCXVmyvhqDjJljHMmDLnrj/prjzorz/kt17nvGnZp0XGjBjKjxvQlSTepTfEkjHTnTXpsEHpsUTXojzbpj7SnTTDjSTGkSrIjhzjqjnepzrjrD7lr0biqz/cpTnHjBjKjx3MlCfNlSjJlC3GjiLhqTzls1Djrkbkqz3UnjTCkC++gxC/hRLFjB65gxm9hhvPliffpzjlrT3lrDrPmzTKljKwgy+4gBO/hhWrdxGlcxLFjR7kqTfnrDnorz7jqjvRnj2ndx6xeg6odAy/hxfkqjjqsUHmrkHUoUSxhUG7gxXgpzjRo1CmgEewh00nmQYaAAAAe3RSTlMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAMpetXWfCgBT9/9/d2TQg1v88FlCG7ZG2z+1xsOi9cbASl3xvbXGxHA1xwV0NgdFc/YHRrR2R004+ozJaHF1PauIQoaMmrr9vf0sk8MAQgcWnic+fGtSwsQPrm/UQuNPWZaAAABG0lEQVQY0wEQAe/+AAAAAAEdHh8gISIjJAIDBAUAAAAABiUmJ3t8KCkqKywHCAAAAAAJLX1+f4CBgoMuLzAxAAAAAAoyhIWGh4iJiouMMzQAAAsMDTWNjo+QkZI2k5Q3OAAODxA5OpWWl5iZmpucnTs8AD0+P0BBnp+goaKjpKWmQkMAREWnqKmqq6ytrq+wsYpGRwBISbKztLW2t7i5uru8vUpLAExNvr/AwcLDxMXGx8jJTk8AUFHKy8zNzs/Q0dKb09RSUwBUVdXW19jZ2tud3N3e31ZXAFhZWlvg4eLj5OXm5+hcXV4AX2BhYmNkZenq6+xmZ2hpEQAAamtsbW5vcO1xcnN0EhMUABUWABdqdXZ3eHl6GBkaGxxTKXBYeDUm8QAAAABJRU5ErkJggg==" alt="Crates.io"></a>
    <a href="https://github.com/v3xlabs/eth-prices"><img src="https://img.shields.io/badge/Repository-v3xlabs/eth--prices-blue?style=flat" alt="Repository"></a>
    <a href="#"><img src="https://img.shields.io/badge/Status-In%20Development-blue?style=flat" alt="Status: In Development"></a>
    <a href="#"><img src="https://img.shields.io/badge/License-LGPL--3.0-hotpink?style=flat" alt="License: LGPL-3.0"></a>
</p>

> [!IMPORTANT]
> eth-prices aims to provide Ethereum price estimation only. It is not intended for, nor does it provide guarantees of, exchange rates.

## Quickstart

```sh
cargo add eth-prices
```

```rust
use eth_prices::{
    quoter::{Quoter, RateDirection},
    asset::Asset,
};

// Define your data sources
let quoter = vec![
    UniswapV2Quoter::from_selector(provider, UniswapV2Selector::Pair { pair_address }).await,
    UniswapV3Quoter::from_selector(provider, UniswapV3Selector::Pool { pool_address }).await,
    ERC4626Quoter::new(vault_address, provider).await,
    FixedQuoter::new(fixed_rate, provider).await,
];

// Create a router
let router = QuoterGraph::new(quoters);

// Compute a route
let token_in = TokenIdentifier::ERC20 { address: address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48") };
let token_out = TokenIdentifier::Fiat { symbol: "usd".to_string() };
let route = router.compute(&token_in, &token_out).await.unwrap();

// Get the latest block number
let block = provider.get_block_number().await.unwrap();

// Quote the rate
let quote = route.quote(block, amount_in).await.unwrap();

```

## Data Sources

Currently supported data sources include:

- [Uniswap V2](./pkg/src/quoter/uniswap_v2/mod.rs) - for [Uniswap V2](https://uniswap.org/blog/uniswap-v2) pairs
- [Uniswap V3](./pkg/src/quoter/uniswap_v3/mod.rs) - for [Uniswap V3](https://uniswap.org/blog/uniswap-v3) pools
- [Curve](./pkg/src/quoter/curve/mod.rs) - for [Curve](https://curve.finance) StableSwap and crypto pools (mainnet MetaRegistry discovery)
- [ERC-4626](./pkg/src/quoter/erc4626/mod.rs) - for [ERC-4626](https://eips.ethereum.org/EIPS/eip-4626)-compatible vaults (Morpho, Aave, etc.)
- [Fixed](./pkg/src/quoter/fixed/mod.rs) - for static rates, such as WETH to ETH, or USDC to USD
- [European Central Bank](./pkg/src/quoter/ecb/mod.rs) - for fiat exchange rates (uses single HTTP call and requires feature flag `ecb`)
- [Chainlink](./pkg/src/quoter/chainlink/mod.rs) - for [Chainlink Price Feeds](https://docs.chain.link/data-feeds)

## Examples

This crate has a few examples you can toy around with:

- [Uniswap V2 Quoter](./examples/uniswap/) - to quote rates for Uniswap V2 pairs
- [Prometheus Exporter](./examples/prometheus/) - to export price data to metrics
- [Fiat Quoter](./examples/fiat/) - to quote fiat rates
- [Auto Router](./examples/auto_router/) - to automatically discover quoters and build a router

## Autorouter Evaluation

The live evaluation suite in [`bench`](./bench/) compares the Rust and TypeScript
autorouters at the same Ethereum block against six keyless USD price sources.
Run it with `just eval`. Results are written to `target/evals/`.

## Documentation

You can read the documentation at [docs.rs/eth-prices](https://docs.rs/eth-prices).
