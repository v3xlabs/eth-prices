use std::collections::HashMap;

use anyhow::Result;
use petgraph::{
    dot::Dot,
    graph::{NodeIndex, UnGraph},
};
use tracing::info;

use crate::{
    quoter::{Quoter, QuoterInstance},
    router::Route,
    token::TokenIdentifier,
};

#[derive(Debug, Clone)]
pub struct QuoterGraph {
    pub quoters: Vec<QuoterInstance>,
    pub graph: UnGraph<String, String>,
    pub token_map: HashMap<String, NodeIndex<u32>>,
}

impl Default for QuoterGraph {
    fn default() -> Self {
        Self {
            quoters: Vec::new(),
            graph: UnGraph::new_undirected(),
            token_map: HashMap::new(),
        }
    }
}

impl FromIterator<QuoterInstance> for QuoterGraph {
    fn from_iter<T: IntoIterator<Item = QuoterInstance>>(iter: T) -> Self {
        let mut graph = Self::default();
        for quoter in iter {
            graph.add_quoter(&quoter);
            graph.quoters.push(quoter);
        }
        graph
    }
}

impl QuoterGraph {
    pub fn get_token_index(&self, token: &TokenIdentifier) -> Option<NodeIndex<u32>> {
        self.token_map.get(&token.to_string()).copied()
    }

    pub fn get_token_by_index(&self, index: NodeIndex<u32>) -> Option<TokenIdentifier> {
        self.token_map
            .iter()
            .find(|x| *x.1 == index)
            .map(|(token, _)| token.clone())
            .map(TokenIdentifier::try_from)
            .and_then(|x| x.ok())
    }

    pub fn add_token(&mut self, token: &TokenIdentifier) -> NodeIndex<u32> {
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

    pub fn add_quoter(&mut self, quoter: &impl Quoter) {
        let slug = quoter.get_slug();
        let (token_in, token_out) = quoter.get_tokens();

        let token_in_index = self.add_token(&token_in);
        let token_out_index = self.add_token(&token_out);

        self.graph
            .extend_with_edges([(token_in_index, token_out_index, slug)]);
    }

    pub fn to_graphviz(&self) -> String {
        Dot::new(&self.graph).to_string()
    }

    /// compute a route given an input and output token
    pub fn compute(
        &self,
        input_token: &TokenIdentifier,
        output_token: &TokenIdentifier,
    ) -> Result<Route> {
        let token_a_index = self
            .get_token_index(input_token)
            .ok_or(anyhow::anyhow!("Token not found"))?;
        let token_b_index = self
            .get_token_index(output_token)
            .ok_or(anyhow::anyhow!("Token not found"))?;

        info!(
            target: "router::compute_start",
            input_token = %input_token,
            output_token = %output_token,
        );

        let path = petgraph::algo::astar(
            &self.graph,
            token_a_index,
            |x| x == token_b_index,
            |_| 0,
            |_| 0,
        );

        match path {
            None => Err(anyhow::anyhow!("No path found")),
            Some((_cost, node_path)) => {
                info!(
                    target: "router::compute_end",
                    node_path = ?node_path,
                );
                let token_route = node_path
                    .iter()
                    .map(|x| self.get_token_by_index(*x).unwrap())
                    .collect::<Vec<TokenIdentifier>>();

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

                Ok(Route {
                    path,
                    input_token: input_token.clone(),
                    output_token: output_token.clone(),
                })
            }
        }
    }
}
