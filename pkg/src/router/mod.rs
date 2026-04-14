use std::sync::Arc;

use crate::Result;
use alloy::primitives::{BlockNumber, U256};
use tracing::info;

use crate::{
    quoter::{Quoter, QuoterInstance, RateDirection},
    token::TokenIdentifier,
};

pub mod graph;

#[derive(Debug, Clone)]
pub struct RouteStep {
    pub quoter: Arc<QuoterInstance>,
    pub direction: RateDirection,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub path: Vec<RouteStep>,
    pub input_token: TokenIdentifier,
    pub output_token: TokenIdentifier,
}

impl Route {
    /// calculate a quote for a given route
    pub async fn quote(&self, block: BlockNumber, amount_in: U256) -> Result<U256> {
        let mut amount_out = amount_in;

        for step in self.path.iter() {
            let quoter_slug = step.quoter.to_string();

            info!(
                target: "router::quote_start",
                quoter_slug,
                amount_in = %amount_in,
                direction = %step.direction,
            );

            let rate = step.quoter.rate(amount_out, step.direction, block).await?;
            amount_out = rate;

            info!(
                target: "router::quote_end",
                quoter_slug,
                amount_in = %amount_in,
                direction = %step.direction,
                amount_out = %amount_out,
            );
        }

        Ok(U256::from(amount_out))
    }
}
