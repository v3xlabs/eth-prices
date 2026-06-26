use std::time::{SystemTime, UNIX_EPOCH};

use alloy::primitives::U256;
use eth_prices::{asset::AssetIdentifier, network::NetworkInstant, quoter::ecb::EcbRateSource};

#[tokio::main]
pub async fn main() {
    println!("Hello, world!");

    let fiat_graph = EcbRateSource::default().graph();

    let token_in = AssetIdentifier::Fiat {
        symbol: "czk".to_string(),
    };
    let token_out = AssetIdentifier::Fiat {
        symbol: "sek".to_string(),
    };
    let network = NetworkInstant::default().with_fiat_timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );

    let route = fiat_graph.compute(&token_in, &token_out).unwrap();

    let quote = route.quote(&network, U256::from(1_000_000)).await.unwrap();

    println!("quote: {:?}", quote);
}
