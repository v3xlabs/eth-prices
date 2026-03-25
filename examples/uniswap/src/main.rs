use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder},
};
use eth_prices::{
    quoter::{
        Quoter, RateDirection,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
    },
    token::Token,
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
    let token_a = Token::new(token_a, &provider).await.unwrap();
    let token_b = Token::new(token_b, &provider).await.unwrap();
    let amount_in = token_a.nominal_amount().await;
    let block = provider.get_block_number().await.unwrap();
    let rate = quoter
        .rate(amount_in, RateDirection::Forward, block)
        .await
        .unwrap();

    println!(
        "rate: {} {} = {} {}",
        token_a.format_amount(amount_in, 4).await.unwrap(),
        token_a.symbol,
        token_b.format_amount(rate, 4).await.unwrap(),
        token_b.symbol
    );
}
