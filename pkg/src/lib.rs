/*!
`eth-prices` is a pricing library & routing engine for EVM assets.

This crate currently exposes protocol-specific quoters that can read a rate at a
specific block height.

# Overview

Here is a simple example showing off some of the features of `eth-prices`:
```rust
use eth_prices::{quoter::Quoter, router::Router, asset::AssetIdentifier, network::NetworkTime, quoter::uniswap_v3::{UniswapV3Quoter, factory::UniswapV3Selector}};
use alloy::primitives::{address, U256};
use alloy::providers::{ProviderBuilder, Provider};

#[tokio::main]
pub async fn main() {
    println!("Hello, world!");
    let provider = ProviderBuilder::new().connect("https://ethereum-rpc.publicnode.com").await.unwrap().erased();
    let selector = UniswapV3Selector::Pool { pool_address: address!("0x99ac8ca7087fa4a2a1fb6357269965a2014abc35") };
    let quoter = UniswapV3Quoter::from_selector(&provider, selector).await.unwrap();
    let router = Router::from_iter(vec![quoter.into()]);

    let token_in = AssetIdentifier::try_from("0x2260fac5e5542a773aa44fbcfedf7c193bc2c599").unwrap();
    let token_out = AssetIdentifier::try_from("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
    let route = router.compute(&token_in, &token_out).unwrap();

    let block = provider.get_block_number().await.unwrap();
    let network = NetworkTime::EVM(1, block, provider).instant();
    let amount = U256::from(1_000_000);
    let quote = route.quote(&network, amount).await.unwrap();

    println!("quote: {:?}", quote);
}
```

Today, the main building blocks are:
- [`quoter::Quoter`] for single-hop quote sources.
- [`router::Router`] for routing between assets.
- [`asset::AssetIdentifier`] for identifying ERC-20, fiat, and native assets.
- [`asset::Asset`] for asset metadata and amount formatting helpers.
- [`network::NetworkInstant`] for managing network times and providers.

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

pub mod asset;
pub mod config;
pub mod network;
pub mod provider;
pub mod quoter;
pub mod router;

#[cfg(target_arch = "wasm32")]
pub mod js;

#[cfg(test)]
pub mod tests;
