//! Builder API for constructing routers from loaded quoters.
//!
//! This provides a simple way to build a QuoterGraph from quoters that have already
//! been loaded/created. The loading itself is handled separately (e.g., by config
//! or by wasm bindings).

use crate::router::graph::QuoterGraph;

/// Consumes a collection of initialized quoters to build a routing [`QuoterGraph`].
///
/// This is useful when quoters are created externally (for example from config
/// loading or bindings) and then assembled into a routing graph.
///
/// # Example
///
/// ```rust
/// use eth_prices::{
///     builder::build_graph,
///     quoter::{fixed::FixedQuoter, QuoterInstance},
/// };
///
/// let quoter = FixedQuoter {
///     token_in: "0x0000000000000000000000000000000000000001".to_string().try_into().unwrap(),
///     token_out: "fiat:usd".to_string().try_into().unwrap(),
///     fixed_rate: 1.0,
/// };
///
/// let graph = build_graph(vec![QuoterInstance::Fixed(quoter)]);
/// assert_eq!(graph.quoters.len(), 1);
/// ```
pub fn build_graph(
    quoters: impl IntoIterator<Item = crate::quoter::QuoterInstance>,
) -> QuoterGraph {
    QuoterGraph::from_iter(quoters)
}
