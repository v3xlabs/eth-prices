//! Historical token pricing primitives for EVM assets.
//!
//! This crate currently exposes protocol-specific quoters that can read a rate at a
//! specific block height. The long-term goal is to build a reusable routing layer on top
//! of these quoters so callers can resolve multi-hop prices such as
//! `wBTC -> wETH -> USDC -> fiat:usd` once and reuse that route across many blocks.
//!
//! Today, the main building blocks are:
//! - [`quoter::Quoter`] for single-hop quote sources.
//! - [`quoter::QuoterInstance`] for storing heterogeneous quote sources together.
//! - [`token::TokenIdentifier`] for identifying ERC-20, fiat, and native assets.
//! - [`token::Token`] for token metadata and amount formatting helpers.
//!
//! # Quoters
//!
//! Currently supported quoters include:
//! - [`quoter::fixed`] for static conversion rates.
//! - [`quoter::uniswap_v2`] for Uniswap v2 pairs.
//! - [`quoter::uniswap_v3`] for Uniswap v3 pools.
//! - [`quoter::erc4626`] for ERC-4626 vaults.
//!  
//! # Routing
//! 
//! Routing is currently in-progress and will be available in a future release.

pub mod token;
pub mod quoter;
pub mod router;
pub mod config;

// Utilities for testing and development
#[cfg(test)]
pub mod tests;
