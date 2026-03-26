use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    sync::{Arc, atomic::AtomicU64},
};
use tracing::info;


use eth_prices::{
    quoter::{Quoter, QuoterInstance, RateDirection},
    token::Token,
};

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    
    // Define your data sources
    let quoters: Vec<QuoterInstance> = vec![
        "uniswap_v2:0xB87b65Dacc6171B3ca8c4A934601d0FcB6c61049", // wETH/ENS
        "uniswap_v2:0x4028DAAC072e492d34a3Afdbef0ba7e35D8b55C4", // stEth/wETH
        "uniswap_v3:0x99ac8ca7087fa4a2a1fb6357269965a2014abc35", // wBTC/USDC
        "uniswap_v3:0xdceaf5d0e5e0db9596a47c0c4120654e80b1d706", // Aave/USDC
        "uniswap_v3:0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8", // USDC/wETH
        "uniswap_v3:0x6546055f46e866a4b9a4a13e81273e3152bae5da", // xAUT/USDT
        "erc4626:0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1", // stETH/wETH
        "fixed:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48:fiat:usd:1.0", // 1:1 USDC/USD
    ].into_iter().map(|s| QuoterInstance::from_str(s).unwrap()).collect();
    
    // Create a router
    let router = QuoterGraph::new(quoters);
    
    // Compute a route
    let token_in: TokenIdentifier = "erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into();
    let token_out: TokenIdentifier = "fiat:usd".into();
    let route = router.compute(&token_in, &token_out).await.unwrap();
    
    // Get the latest block number
    let block = provider.get_block_number().await.unwrap();
    
    // Quote the rate
    let quote = route.quote(block, amount_in).await.unwrap();    
}
