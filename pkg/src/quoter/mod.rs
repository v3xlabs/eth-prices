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
pub use quoter::Quoter;

// Quoters

#[cfg(feature = "ecb")]
pub mod ecb;
pub mod erc4626;
pub mod fixed;
pub mod quoter;
pub mod uniswap_v2;
pub mod uniswap_v3;
