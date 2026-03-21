use std::collections::HashMap;

use petgraph::{
    dot::Dot,
    graph::{NodeIndex, UnGraph},
};

use crate::{quoter::{Quoter, QuoterInstance}, token::TokenIdentifier};

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
}
