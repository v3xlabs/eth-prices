<h1 align="center">
    eth-prices
</h1>

<p align="center">
  A smol rust crate for fetching Ethereum asset prices directly from rpc.
</p>
<p align="center">
    <a href="https://github.com/v3xlabs/eth-prices"><img src="https://img.shields.io/badge/Repository-v3xlabs/eth--prices-blue?style=flat" alt="Repository"></a>
    <a href="#"><img src="https://img.shields.io/badge/Status-In%20Development-blue?style=flat" alt="Status: In Development"></a>
    <a href="#"><img src="https://img.shields.io/badge/License-LGPL--3.0-hotpink?style=flat" alt="License: LGPL-3.0"></a>
</p>

## Quickstart

```sh
cargo add eth-prices
```

```rust
use eth_prices::{
    quoter::{Quoter, RateDirection},
    token::Token,
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

## Examples

This crate has a few examples you can toy around with:

- [Uniswap V2 Quoter](./examples/uniswap/)
- [Prometheus Exporter](./examples/prometheus/)
