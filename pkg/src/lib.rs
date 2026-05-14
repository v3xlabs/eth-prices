/*!
`eth-prices` is a pricing library & routing engine for EVM assets.

This crate currently exposes protocol-specific quoters that can read a rate at a
specific block height.

# Overview

Here is a simple example showing off some of the features of `eth-prices`:
```rust
use eth_prices::{quoter::Quoter, router::Router};
use alloy::primitives::address;

let quoter = UniswapV3Quoter::from_pool(address!("0x1234567890123456789012345678901234567890")).await;
let router = Router::from_iter(vec![quoter]);

let token_in: AssetIdentifier = "erc20:0x1234567890123456789012345678901234567890".try_into().unwrap();
let token_out: AssetIdentifier = "erc20:0x1234567890123456789012345678901234567890".try_into().unwrap();
let route = router.compute(&token_in, &token_out).await.unwrap();

let block = provider.get_block_number().await.unwrap();
let network = Network::EVM(1, block, provider);
let amount = U256::from(1_000_000);
let quote = route.quote(&network, amount).await.unwrap();

println!("quote: {:?}", quote);
```

Today, the main building blocks are:
- [`quoter::Quoter`] for single-hop quote sources.
- [`router::Router`] for routing between assets.
- [`asset::AssetIdentifier`] for identifying ERC-20, fiat, and native assets.
- [`asset::Asset`] for asset metadata and amount formatting helpers.

# Quoters

Currently supported quoters include:
- [`quoter::fixed`] for static conversion rates.
- [`quoter::uniswap_v2`] for Uniswap v2 pairs.
- [`quoter::uniswap_v3`] for Uniswap v3 pools.
- [`quoter::erc4626`] for ERC-4626 vaults.
- [`quoter::ecb`] for European Central Bank (ECB) rates.

# Features

- `ecb` - Enable European Central Bank (ECB) quoters.

# Routing

Use the [`router::Router`] struct to compute a route and quote the rate.

# Examples

You can find more examples in the [examples](https://github.com/v3xlabs/eth-prices/tree/master/examples) directory.
*/

pub mod error;
pub use error::{EthPricesError, Result};

pub mod config;
pub mod quoter;
pub mod router;
pub mod asset;
pub mod network;

#[cfg(target_arch = "wasm32")]
pub mod js;

#[cfg(test)]
pub mod tests;
