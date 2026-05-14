use std::fmt::{self, Debug, Display};

use alloy::primitives::U256;

use crate::{Result, network::Network, quoter::RateDirection, token::identity::TokenIdentifier};

/// A single-hop quote source.
///
/// Implementors expose which two assets they connect and can quote an input amount at a
/// specific block height.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Quoter: Send + Sync + Debug {
    fn identity(&self) -> String;

    /// Returns the pair of assets connected by this quoter.
    fn tokens(&self) -> (TokenIdentifier, TokenIdentifier);

    /// Quotes `amount_in` at the provided block height.
    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        network: &Network,
    ) -> Result<U256>;
}

impl Display for dyn Quoter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identity())
    }
}
