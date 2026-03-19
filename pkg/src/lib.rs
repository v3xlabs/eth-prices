//! Historical token pricing primitives for EVM assets.
//!
//! This crate currently exposes protocol-specific quoters that can read a rate at a
//! specific block height. The long-term goal is to build a reusable routing layer on top
//! of these quoters so callers can resolve multi-hop prices such as
//! `wBTC -> wETH -> USDC -> fiat:usd` once and reuse that route across many blocks.
//!
//! Today, the main building blocks are:
//! - [`quoters::Quoter`] for single-hop quote sources.
//! - [`quoters::QuoterInstance`] for storing heterogeneous quote sources together.
//! - [`token::TokenIdentifier`] for identifying ERC-20, fiat, and native assets.
//! - [`token::Token`] for token metadata and amount formatting helpers.
//!
//! Currently supported quoters include:
//! - [`quoters::fixed::FixedTracker`] for static conversion rates.
//! - [`quoters::uniswap_v2::UniswapV2Quoter`] for Uniswap v2 pairs.
//! - [`quoters::uniswap_v3::UniswapV3Quoter`] for Uniswap v3 pools.
//! - [`quoters::erc4626::ERC4626Quoter`] for ERC-4626 vaults.
//!
//! The public API is still early and intentionally small, but these types are the
//! foundation for the higher-level router API that will sit on top.

pub mod quoters;
#[cfg(test)]
pub mod tests;
pub mod token;
