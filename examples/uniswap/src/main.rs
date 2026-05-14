use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder},
};
use eth_prices::{
    asset::Asset, network::Network, quoter::{
        Quoter, RateDirection,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
    }
};

#[tokio::main]
pub async fn main() {
    println!("Hello, world!");
    let pair_address = address!("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc");
    let provider = ProviderBuilder::new()
        .connect("https://reth-ethereum.ithaca.xyz/rpc")
        .await
        .unwrap()
        .erased();
    let quoter =
        UniswapV2Quoter::from_selector(&provider, UniswapV2Selector::Pair { pair_address })
            .await
            .unwrap();

    let (token_a, token_b) = quoter.tokens();
    let (token_a, token_b) = (
        Asset::new(token_a, &provider).await.unwrap(),
        Asset::new(token_b, &provider).await.unwrap(),
    );
    let amount_in = token_a.nominal_amount();
    let block = provider.get_block_number().await.unwrap();
    let network = Network::EVM(1, block, provider.clone());
    let rate = quoter
        .rate(amount_in, RateDirection::Forward, &network)
        .await
        .unwrap();

    println!(
        "rate: {} {} = {} {}",
        token_a.format_amount(amount_in, 4).unwrap(),
        token_a.symbol,
        token_b.format_amount(rate, 4).unwrap(),
        token_b.symbol
    );
}
