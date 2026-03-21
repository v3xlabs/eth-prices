use alloy::primitives::{BlockNumber, U256};
use anyhow::Result;
use tracing::{Level, event, info, span};

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
    /// compute a route given an input and output token
    pub fn compute(
        graph: &QuoterGraph,
        input_token: &TokenIdentifier,
        output_token: &TokenIdentifier,
    ) -> Result<Self> {
        let token_a_index = graph
            .get_token_index(input_token)
            .ok_or(anyhow::anyhow!("Token not found"))?;
        let token_b_index = graph
            .get_token_index(output_token)
            .ok_or(anyhow::anyhow!("Token not found"))?;

        info!(
            target: "router::compute_start",
            input_token = %input_token,
            output_token = %output_token,
        );

        let path = petgraph::algo::astar(
            &graph.graph,
            token_a_index,
            |x| x == token_b_index,
            |_| 0,
            |_| 0,
        );

        match path {
            None => return Err(anyhow::anyhow!("No path found")),
            Some((_cost, node_path)) => {
                info!(
                    target: "router::compute_end",
                    node_path = ?node_path,
                );
                let token_route = node_path
                    .iter()
                    .map(|x| graph.get_token_by_index(*x).unwrap())
                    .collect::<Vec<_>>();

                let mut path = Vec::new();

                let mut previous_token = input_token;
                for next_token in token_route.iter() {
                    if *previous_token == *next_token {
                        continue;
                    };

                    let quoter = graph
                        .quoters
                        .iter()
                        .find(|x| {
                            let (token_in, token_out) = x.get_tokens();

                            (token_in == *previous_token && token_out == *next_token)
                                || (token_in == *next_token && token_out == *previous_token)
                        })
                        .unwrap();

                    path.push(quoter.get_slug());
                    previous_token = next_token;
                }

                if path.len() != node_path.len() - 1 {
                    return Err(anyhow::anyhow!(
                        "Path length mismatch {} != {}",
                        path.len(),
                        node_path.len() - 1
                    ));
                }

                Ok(Self {
                    path,
                    input_token: input_token.clone(),
                    output_token: output_token.clone(),
                })
            }
        }
    }

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
