use std::collections::HashMap;

use petgraph::{
    dot::Dot,
    graph::{NodeIndex, UnGraph},
};
use tracing::info;

use crate::{
    Result,
    asset::AssetIdentifier,
    quoter::{AnyQuoter, RateDirection},
};
use route::{Route, RouteStep};

pub use auto::AutoRouter;

pub mod auto;
pub mod route;

const MAX_CONFIDENCE: u64 = 100;

#[derive(Debug, Clone)]
pub struct Router {
    pub quoters: Vec<AnyQuoter>,
    pub graph: UnGraph<String, String>,
    pub token_map: HashMap<String, NodeIndex<u32>>,
    confidences: HashMap<String, u64>,
}

impl Default for Router {
    fn default() -> Self {
        Self {
            quoters: Vec::new(),
            graph: UnGraph::new_undirected(),
            token_map: HashMap::new(),
            confidences: HashMap::new(),
        }
    }
}

impl FromIterator<AnyQuoter> for Router {
    fn from_iter<T: IntoIterator<Item = AnyQuoter>>(iter: T) -> Self {
        let mut graph = Self::default();
        for quoter in iter {
            graph.add_quoter(quoter);
        }
        graph
    }
}

impl Router {
    pub fn get_token_index(&self, token: &AssetIdentifier) -> Option<NodeIndex<u32>> {
        self.token_map.get(&token.to_string()).copied()
    }

    pub fn get_token_by_index(&self, index: NodeIndex<u32>) -> Option<AssetIdentifier> {
        self.token_map
            .iter()
            .find(|x| *x.1 == index)
            .map(|(token, _)| token.clone())
            .map(AssetIdentifier::try_from)
            .and_then(|x| x.ok())
    }

    pub fn add_token(&mut self, token: &AssetIdentifier) -> NodeIndex<u32> {
        match self.token_map.get(&token.to_string()) {
            Some(node_index) => *node_index,
            None => {
                let slug = token.to_string();
                let node_index = self.graph.add_node(slug.to_owned());
                self.token_map.insert(slug, node_index);
                node_index
            }
        }
    }

    pub fn add_quoter(&mut self, quoter: AnyQuoter) {
        let slug = quoter.to_string();
        let confidence = quoter.confidence;
        let (token_in, token_out) = quoter.tokens();
        self.quoters.push(quoter);
        self.confidences.insert(slug.clone(), confidence);

        let token_in_index = self.add_token(&token_in);
        let token_out_index = self.add_token(&token_out);

        self.graph
            .extend_with_edges([(token_in_index, token_out_index, slug)]);
    }

    #[cfg(feature = "ecb")]
    pub fn with_ecb(mut self) -> Self {
        use crate::quoter::ecb::EcbRateSource;

        let fiat = EcbRateSource::default();
        self.merge_with(fiat.graph());
        self
    }

    /// Merge two routers together.
    ///
    /// This can be useful when leveraging [`crate::quoter::ecb::EcbRateSource`] to build a router.
    /// ```
    /// use eth_prices::{quoter::ecb::EcbRateSource, router::Router};
    ///
    /// let ecb_rate_source = EcbRateSource::default();
    /// let ecb_graph = ecb_rate_source.graph();
    ///
    /// let router = Router::default().merge_with(ecb_graph);
    /// ```
    pub fn merge_with(&mut self, other: Self) -> &mut Self {
        for quoter in other.quoters {
            self.add_quoter(quoter);
        }
        self
    }

    pub fn to_graphviz(&self) -> String {
        Dot::new(&self.graph).to_string()
    }

    /// compute a route given an input and output token
    pub fn compute(
        &self,
        input_token: &AssetIdentifier,
        output_token: &AssetIdentifier,
    ) -> Result<Route> {
        let token_a_index = self
            .get_token_index(input_token)
            .ok_or_else(|| crate::error::EthPricesError::AssetNotFound(input_token.to_string()))?;
        let token_b_index = self
            .get_token_index(output_token)
            .ok_or_else(|| crate::error::EthPricesError::AssetNotFound(output_token.to_string()))?;

        info!(
            target: "router::compute_start",
            input_token = %input_token,
            output_token = %output_token,
        );

        let confidences = &self.confidences;
        let path = petgraph::algo::astar(
            &self.graph,
            token_a_index,
            |x| x == token_b_index,
            |edge| {
                let slug = edge.weight();
                let conf = confidences.get(slug.as_str()).copied().unwrap_or(0);
                (MAX_CONFIDENCE + 1 - conf.min(MAX_CONFIDENCE)) as u32
            },
            |_| 0,
        );

        match path {
            None => Err(crate::error::EthPricesError::NoRouteFound(
                input_token.to_string(),
                output_token.to_string(),
            )),
            Some((_cost, node_path)) => {
                info!(
                    target: "router::compute_end",
                    node_path = ?node_path,
                );
                let token_route = node_path
                    .iter()
                    .map(|x| {
                        self.get_token_by_index(*x)
                            .ok_or_else(|| crate::error::EthPricesError::MissingTokenInRoute)
                    })
                    .collect::<Result<Vec<AssetIdentifier>>>()?;

                let mut path = Vec::new();

                let mut previous_token = input_token;
                for next_token in token_route.iter() {
                    if *previous_token == *next_token {
                        continue;
                    };

                    let quoter = self
                        .quoters
                        .iter()
                        .find(|x| {
                            let (token_in, token_out) = x.tokens();

                            (token_in == *previous_token && token_out == *next_token)
                                || (token_in == *next_token && token_out == *previous_token)
                        })
                        .ok_or_else(|| crate::error::EthPricesError::MissingQuoterInRoute)?;

                    path.push(RouteStep {
                        quoter: quoter.clone(),
                        direction: if *previous_token == quoter.tokens().0 {
                            RateDirection::Forward
                        } else {
                            RateDirection::Reverse
                        },
                    });
                    previous_token = next_token;
                }

                if path.len() != node_path.len() - 1 {
                    return Err(crate::error::EthPricesError::PathLengthMismatch {
                        expected: node_path.len() - 1,
                        actual: path.len(),
                    });
                }

                Ok(Route {
                    path,
                    input_token: input_token.clone(),
                    output_token: output_token.clone(),
                })
            }
        }
    }
}
