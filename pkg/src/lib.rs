/*!
`eth-prices` is a pricing library & routing engine for EVM assets.

This crate currently exposes protocol-specific quoters that can read a rate at a
specific block height.

# Overview

Here is a simple example showing off some of the features of `eth-prices`:
```rust,ignore
use eth_prices::{quoter::Quoter, token::Token};
use alloy::primitives::address;

let quoter = UniswapV3Quoter::from_pool(address!());
```

Today, the main building blocks are:
- [`quoter::Quoter`] for single-hop quote sources.
- [`quoter::QuoterInstance`] for storing heterogeneous quote sources together.
- [`token::TokenIdentifier`] for identifying ERC-20, fiat, and native assets.
- [`token::Token`] for token metadata and amount formatting helpers.
# Quoters
Currently supported quoters include:
- [`quoter::fixed`] for static conversion rates.
- [`quoter::uniswap_v2`] for Uniswap v2 pairs.
- [`quoter::uniswap_v3`] for Uniswap v3 pools.
- [`quoter::erc4626`] for ERC-4626 vaults.

# Routing

Routing is currently in-progress and will be available in a future release.

# Examples

You can find more examples in the [examples](https://github.com/v3xlabs/eth-prices/tree/master/examples) directory.
*/

pub mod config;
pub mod quoter;
pub mod router;
pub mod token;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Utilities for testing and development
#[cfg(test)]
pub mod tests;
