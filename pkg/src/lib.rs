//! Historical token pricing primitives for EVM assets.
//!
//! This crate currently exposes protocol-specific quoters that can read a rate at a
//! specific block height. The long-term goal is to build a reusable routing layer on top
//! of these quoters so callers can resolve multi-hop prices such as
//! `wBTC -> wETH -> USDC -> fiat:usd` once and reuse that route across many blocks.
//!
//! Today, the main building blocks are:
//! - [`trackers::Quoter`] for single-hop quote sources.
//! - [`trackers::QuoterInstance`] for storing heterogeneous quote sources together.
//! - [`token::local::LocalTokenOrFiat`] for identifying ERC-20 assets and fiat endpoints.
//! 
//! Currently supported quoters include:
//! - [`trackers::fixed::FixedTracker`] for static conversion rates.
//! - [`trackers::uniswap_v2::quoter::UniswapV2Quoter`] for Uniswap v2 pairs.
//! - [`trackers::uniswap_v3::quoter::UniswapV3Quoter`] for Uniswap v3 pools.
//! - [`trackers::erc4626::ERC4626Quoter`] for ERC-4626 vaults (Morpho, Aave, etc).
//!
//! The public API is still early and intentionally small, but these types are the
//! foundation for the higher-level router API that will sit on top.

pub mod quoters;
pub mod token;
#[cfg(test)]
pub mod tests;
