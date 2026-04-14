use std::collections::HashSet;

use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder},
};
use tracing::info;

use eth_prices::{
    Result,
    config::Config,
    quoter::{Quoter, RateDirection},
    router::graph::QuoterGraph,
    token::{Token, TokenIdentifier},
};

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load("config.toml").await;

    for (_, chain_config) in config?.chains {
        let url = chain_config.rpc_url;
        let provider = ProviderBuilder::new().connect(&url).await?;

        for token_config in chain_config.tokens {
            let token_address = token_config.address;
            info!("token: {:?}", token_address);
        }

        let box_provider = Box::new(provider.erased());

        let block = box_provider.get_block_number().await?;

        let precision = 10;

        let quoters = chain_config.quoters.all(&box_provider).await?;
        for quoter in &quoters {
            info!("quoter: {:?}", quoter.to_string());
            let (token_a, token_b) = quoter.tokens();

            let token_a = Token::new(token_a, &box_provider).await?;
            let token_b = Token::new(token_b, &box_provider).await?;

            let amount_a = token_a.nominal_amount().await;
            let amount_b = token_b.nominal_amount().await;

            let forward_rate = quoter.rate(amount_a, RateDirection::Forward, block).await?;
            let reverse_rate = quoter.rate(amount_b, RateDirection::Reverse, block).await?;
            info!(
                "forward_rate: {:?} {} = {:?} {}",
                token_a.format_amount(amount_a, precision),
                token_a.symbol,
                token_b.format_amount(forward_rate, precision),
                token_b.symbol,
            );
            info!(
                "reverse_rate: {:?} {} = {:?} {}",
                token_b.format_amount(amount_b, precision),
                token_b.symbol,
                token_a.format_amount(reverse_rate, precision),
                token_a.symbol,
            );
        }

        let router = QuoterGraph::from_iter(quoters);

        info!("{}", router.to_graphviz());

        let mut all_tokens = HashSet::new();

        for quoter in &router.quoters {
            let (token_in, token_out) = quoter.tokens();
            all_tokens.insert(token_in);
            all_tokens.insert(token_out);
        }

        let token_out = TokenIdentifier::ERC20 {
            address: address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        };
        let token_b = Token::new(token_out.clone(), &box_provider).await?;
        let mut routes = Vec::new();

        for token in all_tokens {
            if token == token_out {
                continue;
            }

            let route = router.compute(&token, &token_out)?;
            info!("route: {:?}", route);
            routes.push(route);
        }

        for route in &routes {
            let token_input = &route.input_token;
            let token_a = Token::new(token_input.clone(), &box_provider).await?;
            let token_input = token_a.nominal_amount().await;

            let token_output = route.quote(block, token_input).await?;
            info!(
                "token_output: 1 {} = {:?}",
                token_a.symbol,
                token_b.format_amount(token_output, precision)
            );
        }
    }
    Ok(())
}
