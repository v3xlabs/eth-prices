use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder},
};
use eth_prices::quoters::{
    Quoter, RateDirection,
    uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
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
        UniswapV2Quoter::from_selector(&provider, UniswapV2Selector::Pair { pair_address }).await;

    let (token_a, token_b) = quoter.get_tokens();
    let amount_in = token_a.nominal_amount(&provider).await;
    let block = provider.get_block_number().await.unwrap();
    let rate = quoter
        .get_rate(amount_in, RateDirection::Forward, block)
        .await;

    println!(
        "rate: {} {} = {} {}",
        token_a.format_amount(amount_in, 4, &provider).await,
        token_a.symbol(&provider).await,
        token_b.format_amount(rate, 4, &provider).await,
        token_b.symbol(&provider).await
    );
}
