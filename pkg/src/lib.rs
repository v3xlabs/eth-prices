/*!
`eth-prices` is a price routing and quoting engine for Ethereum assets.

Given a set of on-chain quote sources, the engine finds a path between any two
assets and quotes a conversion at a specific block height, including multi-hop
routes that traverse intermediate tokens.

# Building blocks

- [`quoter::Quoter`] is the trait implemented by every quote source.
- [`quoter::QuoterInstance`] stores heterogeneous quoters together.
- [`token::TokenIdentifier`] identifies ERC-20, fiat, and native assets.
- [`router::graph::QuoterGraph`] discovers routes across the available quoters.
- [`builder::build_graph`] assembles a [`router::graph::QuoterGraph`] from a
  collection of initialized quoters.

# Supported quoters

- [`quoter::fixed`] static conversion rates.
- [`quoter::uniswap_v2`] spot quotes from Uniswap v2 pairs.
- [`quoter::uniswap_v3`] spot quotes from Uniswap v3 pools.
- [`quoter::erc4626`] share/asset conversions for ERC-4626 vaults.

# Example

This example builds a two-hop routing graph with fixed rates and quotes a
conversion from token A to USD.

```rust
use alloy::primitives::U256;
use eth_prices::{
    builder::build_graph,
    quoter::{fixed::FixedQuoter, QuoterInstance},
    token::TokenIdentifier,
};
use futures::executor::block_on;

let token_a: TokenIdentifier = "0x0000000000000000000000000000000000000001".to_string().try_into().unwrap();
let token_b: TokenIdentifier = "0x0000000000000000000000000000000000000002".to_string().try_into().unwrap();
let token_c: TokenIdentifier = "fiat:usd".to_string().try_into().unwrap();

let ab = FixedQuoter {
    token_in: token_a.clone(),
    token_out: token_b.clone(),
    fixed_rate: 2.0,
};
let bc = FixedQuoter {
    token_in: token_b.clone(),
    token_out: token_c.clone(),
    fixed_rate: 3.0,
};

let graph = build_graph(vec![QuoterInstance::Fixed(ab), QuoterInstance::Fixed(bc)]);
let route = graph.compute(&token_a, &token_c).unwrap();

let amount_out = block_on(route.quote(0, U256::from(10))).unwrap();
assert_eq!(amount_out, U256::from(60));
```
*/

pub mod error;
pub use error::{EthPricesError, Result};

pub mod builder;
pub mod config;
pub mod quoter;
pub mod router;
pub mod token;

#[cfg(target_arch = "wasm32")]
pub mod js;

#[cfg(test)]
pub mod tests;
