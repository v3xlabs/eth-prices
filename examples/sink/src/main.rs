use alloy::{
    primitives::{U256, address},
    providers::{Provider, ProviderBuilder},
};
use eth_prices::{
    asset::{Asset, AssetIdentifier}, network::NetworkInstant, provider::RpcProvider, quoter::{
        chainlink::ChainlinkQuoter, fixed::FixedQuoter,
    }, router::{AutoRouter, Router},
};

#[tokio::main]
pub async fn main() {
    let provider = setup().await;

    let mut router = Router::default().with_ecb();

    // Fixed rate: 1 USDC = 1 USD
    router.add_quoter(
        FixedQuoter {
            token_in: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
                .try_into()
                .unwrap(),
            token_in_decimals: 6,
            token_out: "fiat:usd".try_into().unwrap(),
            token_out_decimals: 6,
            fixed_rate: U256::from(100),
            fixed_rate_decimals: 2,
        }
        .into(),
    );

    let erc20s: Vec<AssetIdentifier> = vec![
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
            .try_into()
            .unwrap(),
        "0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1"
            .try_into()
            .unwrap(),
        "0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c"
            .try_into()
            .unwrap(),
        "0xdAC17F958D2ee523a2206206994597C13D831ec7"
            .try_into()
            .unwrap(),
    ];

    let auto = AutoRouter::new(provider.clone(), erc20s.clone())
        .build()
        .await
        .unwrap();

    router.merge_with(auto);

    let token_out = "fiat:usd".try_into().unwrap();

    let network = NetworkInstant::default()
        .with_evm_latest(1.into(), provider.clone())
        .await
        .unwrap()
        .with_now()
        .unwrap();

    // Quote all erc20s
    for token_in in erc20s {
        let asset_in = Asset::new(token_in.clone(), &provider).await.unwrap();
        let amount = asset_in.nominal_amount();
        let route = router.compute(&token_in, &token_out).unwrap();
        let quote = route.quote(&network, amount).await.unwrap();
        println!("quote: {:?}", quote);
    }

    // add chainlink
    let chainlink = ChainlinkQuoter::new(
        address!("0x4ffC43a60e009B551865A93d232E33Fce9f01507"),
        "solana".try_into().unwrap(),
        None,
        "solana".try_into().unwrap(),
        None,
        &provider
    ).await.unwrap();

    router.add_quoter(chainlink.into());

    // Quote solana
    let amount = U256::from(1_000_000_000);
    let route = router.compute(&"solana".try_into().unwrap(), &token_out).unwrap();
    let quote = route.quote(&network, amount).await.unwrap();
    println!("quote external: {:?}", quote);
}

async fn setup() -> RpcProvider {
    ProviderBuilder::new()
        .connect("https://ethereum.reth.rs/rpc")
        .await
        .unwrap()
        .erased()
}
