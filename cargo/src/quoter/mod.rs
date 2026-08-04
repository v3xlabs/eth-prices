/*!
Quote sources for converting one asset into another at a given block height.

A quoter is a single-hop pricing primitive. Examples include a fixed fiat peg,
an on-chain Uniswap pool, or an ERC-4626 vault conversion.
The [`Quoter`] trait is implemented by all supported data sources.
*/

// Submodules

pub mod any;
pub mod direction;
pub use any::AnyQuoter;
pub use direction::RateDirection;

// Quoters

pub mod chainlink;
#[cfg(feature = "ecb")]
pub mod ecb;
pub mod erc4626;
pub mod fixed;
pub mod uniswap_v2;
pub mod uniswap_v3;

// Quoter Trait

use std::fmt::{self, Debug, Display};

use alloy::primitives::U256;

use crate::{Result, asset::identity::AssetIdentifier, network::NetworkInstant};

/// A single-hop quote source.
///
/// Implementors expose which two assets they connect and can quote an input amount at a
/// specific block height.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Quoter: Send + Sync + Debug {
    fn identity(&self) -> String;

    /// Returns the pair of assets connected by this quoter.
    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier);

    /// Quotes `amount_in` at the provided block height.
    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        networks: &NetworkInstant,
    ) -> Result<U256>;
}

impl Display for dyn Quoter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identity())
    }
}
