use alloy::providers::{Provider, ProviderBuilder};

use crate::{
    config::Config,
    quoters::{Quoter, RateDirection},
};

pub mod config;
pub mod quoters;
#[cfg(test)]
pub mod tests;
pub mod token;

#[tokio::main]
pub async fn main() {
    println!("Hello, world!");

    let config = Config::load("config.toml").await;

    for (chain_slug, chain_config) in config.chains {
        println!("chain: {:?}", chain_slug);
        let url = chain_config.rpc_url;
        let provider = ProviderBuilder::new().connect(&url).await.unwrap();

        println!("tokens: {:?}", chain_config.tokens.len());
        for token_config in chain_config.tokens {
            let token_address = token_config.address;
            println!("token: {:?}", token_address);
        }

        let box_provider = Box::new(provider.erased());

        let block = box_provider.get_block_number().await.unwrap();

        let precision = 10;

        // TODO: turn all trackers into quoters
        for quoter in chain_config.trackers.all(&box_provider).await {
            println!("quoter: {:?}", quoter.get_slug());
            let (token_a, token_b) = quoter.get_tokens();
            let amount_a = token_a.nominal_amount(&box_provider).await;
            let amount_b = token_b.nominal_amount(&box_provider).await;

            let forward_rate = quoter
                .get_rate(amount_a, RateDirection::Forward, block)
                .await;
            let reverse_rate = quoter
                .get_rate(amount_b, RateDirection::Reverse, block)
                .await;
            println!(
                "forward_rate: {:?} {} = {:?} {}",
                token_a
                    .format_amount(amount_a, precision, &box_provider)
                    .await,
                token_a.symbol(&box_provider).await,
                token_b
                    .format_amount(forward_rate, precision, &box_provider)
                    .await,
                token_b.symbol(&box_provider).await
            );
            println!(
                "reverse_rate: {:?} {} = {:?} {}",
                token_b
                    .format_amount(amount_b, precision, &box_provider)
                    .await,
                token_b.symbol(&box_provider).await,
                token_a
                    .format_amount(reverse_rate, precision, &box_provider)
                    .await,
                token_a.symbol(&box_provider).await
            );
        }
    }
}
