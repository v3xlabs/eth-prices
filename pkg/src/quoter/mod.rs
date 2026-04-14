//! Quote sources for converting one asset into another at a given block height.
//!
//! A quoter is a single-hop pricing primitive. Examples include a fixed fiat peg,
//! an on-chain Uniswap pool, or an ERC-4626 vault conversion.
//!
//! The [`Quoter`] trait is implemented by all supported data sources.
//!
//! ```rust,ignore
//! use eth_prices::quoter::{fixed::FixedQuoter, Quoter, RateDirection};
//!
//! async {
//!
//! // Create a quoter for a data source
//! let quoter = FixedQuoter::new(config, provider).await;
//!
//! // Get the token pair data
//! let (token_a, token_b) = quoter.tokens();
//! let token_a = Token::new(token_a, provider).await.unwrap();
//! let token_b = Token::new(token_b, provider).await.unwrap();
//!
//! // Inputs
//! let amount_in = token_a.nominal_amount().await;
//! let block = provider.get_block_number().await.unwrap();
//!
//! // Quote the rate
//! let rate = quoter.rate(amount_in, RateDirection::Forward, block).await.unwrap();
//!
//! // Print the rate
//! let rate_formatted = token_b.format_amount(rate, 4).unwrap();
//! println!("rate: {token_a.symbol} = {rate_formatted} {token_b.symbol}");
//! }
//! ```
//!

use std::{
    fmt::{self, Debug, Display},
    ops::Deref,
    sync::Arc,
};

use crate::Result;
use alloy::primitives::{BlockNumber, U256};

use crate::token::identity::TokenIdentifier;

pub mod erc4626;
pub mod fixed;
pub mod uniswap_v2;
pub mod uniswap_v3;

/// The direction to quote along a quoter edge.
///
/// `Forward` means `token0 -> token1` for the pair returned by [`Quoter::get_tokens`].
/// `Reverse` means the inverse direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateDirection {
    Forward,
    Reverse,
}

impl Display for RateDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A single-hop quote source.
///
/// Implementors expose which two assets they connect and can quote an input amount at a
/// specific block height.
#[async_trait::async_trait]
pub trait Quoter: Send + Sync + Debug + Display {
    /// Returns the pair of assets connected by this quoter.
    fn tokens(&self) -> (TokenIdentifier, TokenIdentifier);

    /// Quotes `amount_in` at the provided block height.
    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> Result<U256>;
}

#[derive(Debug, Clone)]
pub struct AnyQuoter(pub Arc<dyn Quoter>);

impl Deref for AnyQuoter {
    type Target = Arc<dyn Quoter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ToQuoter {
    fn strip(self) -> AnyQuoter;
}
