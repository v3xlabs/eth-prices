//! Quote sources for converting one asset into another at a given block height.
//!
//! A quoter is a single-hop pricing primitive. Examples include a fixed fiat peg,
//! an on-chain Uniswap pool, or an ERC-4626 vault conversion.
//! 
//! The [`Quoter`] trait is implemented by all supported data sources.
//! 
//! ```rust,no_run
//! use eth_prices::quoter::Quoter;
//! 
//! // Create a quoter for a data source
//! let quoter = Quoter::new(config, provider).await;
//! 
//! // Get the token pair data
//! let (token_a, token_b) = quoter.get_tokens();
//! let token_a = Token::new(token_a, provider).await.unwrap();
//! let token_b = Token::new(token_b, provider).await.unwrap();
//! 
//! // Inputs
//! let amount_in = token_a.nominal_amount().await;
//! let block = provider.get_block_number().await.unwrap();
//! 
//! // Quote the rate
//! let rate = quoter.get_rate(amount_in, RateDirection::Forward, block).await.unwrap();
//! 
//! // Print the rate
//! let rate_formatted = token_b.format_amount(rate, 4).await;
//! println!("rate: {token_a.symbol} = {rate_formatted} {token_b.symbol}");
//! ```
//! 

use std::fmt::{self, Display};

use alloy::primitives::{BlockNumber, U256};
use anyhow::Result;

use crate::{
    quoter::{
        erc4626::ERC4626Quoter, fixed::FixedQuoter, uniswap_v2::UniswapV2Quoter,
        uniswap_v3::UniswapV3Quoter,
    }, token::identity::TokenIdentifier,
};

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
pub trait Quoter: Send + Sync {
    /// Returns the pair of assets connected by this quoter.
    fn get_tokens(&self) -> (TokenIdentifier, TokenIdentifier);

    /// Quotes `amount_in` at the provided block height.
    ///
    /// The output asset is determined by `direction` relative to [`Quoter::get_tokens`].
    async fn get_rate(&self, amount_in: U256, direction: RateDirection, block: BlockNumber)
    -> Result<U256>;

    /// Returns a stable, human-readable identifier for this quoter.
    fn get_slug(&self) -> String;
}

/// An owned enum wrapper around all supported quote source implementations.
#[derive(Debug, Clone)]
pub enum QuoterInstance {
    /// A fixed-rate synthetic quote source.
    Fixed(FixedQuoter),
    /// A Uniswap v2 pair-backed quote source.
    UniswapV2(UniswapV2Quoter),
    /// A Uniswap v3 pool-backed quote source.
    UniswapV3(UniswapV3Quoter),
    /// An ERC-4626 vault-backed quote source.
    ERC4626(ERC4626Quoter),
}

impl Quoter for QuoterInstance {
    fn get_slug(&self) -> String {
        match self {
            QuoterInstance::Fixed(tracker) => tracker.get_slug(),
            QuoterInstance::UniswapV2(quoter) => quoter.get_slug(),
            QuoterInstance::UniswapV3(quoter) => quoter.get_slug(),
            QuoterInstance::ERC4626(quoter) => quoter.get_slug(),
        }
    }

    fn get_tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        match self {
            QuoterInstance::Fixed(tracker) => tracker.get_tokens(),
            QuoterInstance::UniswapV2(quoter) => quoter.get_tokens(),
            QuoterInstance::UniswapV3(quoter) => quoter.get_tokens(),
            QuoterInstance::ERC4626(quoter) => quoter.get_tokens(),
        }
    }

    async fn get_rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> Result<U256> {
        match self {
            QuoterInstance::Fixed(tracker) => tracker.get_rate(amount_in, direction, block).await,
            QuoterInstance::UniswapV2(quoter) => quoter.get_rate(amount_in, direction, block).await,
            QuoterInstance::UniswapV3(quoter) => quoter.get_rate(amount_in, direction, block).await,
            QuoterInstance::ERC4626(quoter) => quoter.get_rate(amount_in, direction, block).await,
        }
    }
}
