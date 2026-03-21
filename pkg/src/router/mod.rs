use alloy::primitives::{BlockNumber, U256};
use anyhow::Result;
use tracing::info;

use crate::{
    quoter::{Quoter, RateDirection},
    router::graph::QuoterGraph,
    token::TokenIdentifier,
};

pub mod graph;

#[derive(Debug, Clone)]
pub struct Route {
    pub path: Vec<String>,
    pub input_token: TokenIdentifier,
    pub output_token: TokenIdentifier,
}

impl Route {
    /// calculate a quote for a given route
    pub async fn quote(
        &self,
        graph: &QuoterGraph,
        block: BlockNumber,
        amount_in: U256,
    ) -> Result<U256> {
        let mut amount_out = amount_in;

        for quoter in self.path.iter() {
            let quoter = graph
                .quoters
                .iter()
                .find(|x| x.get_slug() == *quoter)
                .unwrap();
            let (token_in, _token_out) = quoter.get_tokens();
            let direction = if token_in == self.input_token {
                RateDirection::Forward
            } else {
                RateDirection::Reverse
            };

            // info!("direction: {:?}", direction);
            // info!("amount_in: {:?}", amount_out);

            let quoter_slug = quoter.get_slug();

            info!(
                target: "router::quote_start",
                quoter_slug,
                amount_in = %amount_in,
                direction = %direction,
            );

            let rate = quoter.get_rate(amount_out, direction, block).await?;
            amount_out = rate;

            info!(
                target: "router::quote_end",
                quoter_slug,
                amount_in = %amount_in,
                direction = %direction,
                amount_out = %amount_out,
            );
        }

        Ok(U256::from(amount_out))
    }
}
