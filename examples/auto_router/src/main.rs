use alloy::providers::{Provider, ProviderBuilder};
use eth_prices::{
    asset::{Asset, AssetIdentifier},
    network::NetworkInstant,
    router::auto::AutoRouter,
};

#[tokio::main]
pub async fn main() {
    println!("=== AutoRouter Example ===\n");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://ethereum.reth.rs/rpc".to_string());

    let provider = ProviderBuilder::new()
        .connect(&rpc_url)
        .await
        .unwrap()
        .erased();

    // Tokens to discover sources for
    let token_addresses: &[(&str, &str)] = &[
        ("wbtc", "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"),
        ("weth", "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
        ("usdc", "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        ("usdt", "0xdac17f958d2ee523a2206206994597c13d831ec7"),
        ("eurc", "0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c"),
        ("ens", "0xC18360217D8F7Ab5e7c516566761Ea12Ce7F9D72"),
        ("aave_usdc", "0x98C23E9d8f34FEFb1B7BD6a91B7FF122F4e16F5c"),
        ("eurc_yield", "0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1"),
    ];

    let tokens: Vec<AssetIdentifier> = token_addresses
        .iter()
        .map(|(_, addr)| AssetIdentifier::try_from(*addr).unwrap())
        .collect();

    println!("Input tokens:");
    for (label, _) in token_addresses {
        println!("  {label}");
    }

    // Discover and build router
    let router = AutoRouter::new(provider.clone(), tokens.clone())
        .with_uniswap()
        .with_sushiswap()
        .with_pancakeswap()
        .build()
        .await
        .unwrap();

    println!("\nDiscovered {} quoter(s):", router.quoters.len());
    for q in &router.quoters {
        let (ta, tb) = q.tokens();
        println!("  {} <-> {}", ta, tb);
    }

    // Route every token towards USDC
    let usdc = AssetIdentifier::try_from("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();

    let block = provider.get_block_number().await.unwrap();
    let networks = NetworkInstant::default().with_evm_block(1.into(), block, provider.clone());

    println!("\n── All routes → USDC ──────────────────────────────────\n");

    for token in &tokens {
        if *token == usdc {
            continue;
        }

        let route = match router.compute(token, &usdc) {
            Ok(r) => r,
            Err(_) => {
                println!("No route found for {} -> USDC", token);
                continue;
            }
        };

        let asset_in = Asset::new(route.input_token.clone(), &provider)
            .await
            .unwrap();
        let asset_out = Asset::new(route.output_token.clone(), &provider)
            .await
            .unwrap();

        let amount_in = asset_in.nominal_amount();
        let amount_out = route.quote(&networks, amount_in).await.unwrap();

        let hops = route.path.len();
        println!(
            "  1 {} = {} {}  ({} hop{})",
            asset_in.symbol,
            asset_out.format_amount(amount_out, 6).unwrap(),
            asset_out.symbol,
            hops,
            if hops == 1 { "" } else { "s" },
        );
    }
}
