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
//! let amount_in = token_a.nominal_amount();
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

// Submodules

pub mod any;
pub mod direction;
pub use any::AnyQuoter;
pub use direction::RateDirection;
pub use quoter::Quoter;

// Quoters

#[cfg(feature = "ecb")]
pub mod ecb;
pub mod erc4626;
pub mod fixed;
pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod quoter;
