//! Builder API for constructing routers from loaded quoters.
//!
//! This provides a simple way to build a QuoterGraph from quoters that have already
//! been loaded/created. The loading itself is handled separately (e.g., by config
//! or by wasm bindings).

use crate::router::graph::QuoterGraph;

/// Build a QuoterGraph from loaded quoters.
pub fn build_graph(
    quoters: impl IntoIterator<Item = crate::quoter::QuoterInstance>,
) -> QuoterGraph {
    QuoterGraph::from_iter(quoters)
}
