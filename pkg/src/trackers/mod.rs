//! Quote sources for converting one asset into another at a given block height.
//!
//! A tracker is a single-hop pricing primitive. Examples include a fixed fiat peg,
//! an on-chain Uniswap pool, or an ERC-4626 vault conversion.

use alloy::{primitives::{BlockNumber, U256}};

use crate::{token::local::LocalTokenOrFiat, trackers::{erc4626::ERC4626Quoter, fixed::FixedTracker, uniswap::{v2::quoter::UniswapV2Quoter, v3::quoter::UniswapV3Quoter}}};

pub mod fixed;
pub mod uniswap;
pub mod erc4626;

/// The direction to quote along a tracker edge.
///
/// `Forward` means `token0 -> token1` for the pair returned by [`Quoter::get_tokens`].
/// `Reverse` means the inverse direction.
pub enum RateDirection {
    Forward,
    Reverse,
}

/// A single-hop quote source.
///
/// Implementors expose which two assets they connect and can quote an input amount at a
/// specific block height.
pub trait Quoter: Send + Sync {
    /// Returns the pair of assets connected by this quoter.
    fn get_tokens(&self) -> (LocalTokenOrFiat, LocalTokenOrFiat);

    /// Quotes `amount_in` at the provided block height.
    ///
    /// The output asset is determined by `direction` relative to [`Quoter::get_tokens`].
    async fn get_rate(&self, amount_in: U256, direction: RateDirection, block: BlockNumber) -> U256;

    /// Returns a stable, human-readable identifier for this quoter.
    fn get_slug(&self) -> String;
}

/// An owned enum wrapper around all supported quote source implementations.
#[derive(Debug, Clone)]
pub enum QuoterInstance {
    Fixed(FixedTracker),
    UniswapV2(UniswapV2Quoter),
    UniswapV3(UniswapV3Quoter),
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

    fn get_tokens(&self) -> (LocalTokenOrFiat, LocalTokenOrFiat) {
        match self {
            QuoterInstance::Fixed(tracker) => tracker.get_tokens(),
            QuoterInstance::UniswapV2(quoter) => quoter.get_tokens(),
            QuoterInstance::UniswapV3(quoter) => quoter.get_tokens(),
            QuoterInstance::ERC4626(quoter) => quoter.get_tokens(),
        }
    }

    async fn get_rate(&self, amount_in: U256, direction: RateDirection, block: BlockNumber) -> U256 {
        match self {
            QuoterInstance::Fixed(tracker) => tracker.get_rate(amount_in, direction, block).await,
            QuoterInstance::UniswapV2(quoter) => quoter.get_rate(amount_in, direction, block).await,
            QuoterInstance::UniswapV3(quoter) => quoter.get_rate(amount_in, direction, block).await,
            QuoterInstance::ERC4626(quoter) => quoter.get_rate(amount_in, direction, block).await,
        }
    }
}
