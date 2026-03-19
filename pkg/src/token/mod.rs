//! Token metadata and identifier helpers.

use alloy::{primitives::{Address, U256}, providers::DynProvider};
use anyhow::Result;

use crate::token::{
    erc20::ERC20,
};

pub mod erc20;
pub mod identity;

pub use identity::TokenIdentifier;

/// A resolved asset with display metadata and decimal precision.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Canonical identifier for the asset.
    pub identifier: TokenIdentifier,
    /// Human-readable asset name.
    pub name: String,
    /// Short symbol used for display.
    pub symbol: String,
    /// Number of decimal places used by on-chain amounts.
    pub decimals: u8,
}

const FIAT_DECIMALS: u8 = 6;

impl Token {
    /// Resolves token metadata for the provided identifier.
    ///
    /// ERC-20 metadata is loaded from chain, while fiat and native assets use local defaults.
    pub async fn new(identifier: TokenIdentifier, provider: &DynProvider) -> Result<Self> {
        let (name, symbol, decimals) = match &identifier {
            TokenIdentifier::ERC20 { address } => {
                let erc20 = ERC20::new(*address, provider);

                (erc20.name().call().await?, erc20.symbol().call().await?, erc20.decimals().call().await?)
            },
            TokenIdentifier::Fiat { symbol } => {

                (symbol.clone(), symbol.clone(), FIAT_DECIMALS)
            },
            TokenIdentifier::Native => {
               ("Native".to_string(), "ETH".to_string(), 18)
            },
        };

        Ok(Self {
            identifier,
            name,
            symbol,
            decimals,
        })
    }

    /// Returns one nominal unit for this token in base precision.
    pub async fn nominal_amount(&self) -> U256 {
        U256::from(10).pow(U256::from(self.decimals))
    }

    /// Formats a raw integer amount into a human-readable decimal string.
    pub async fn format_amount(&self, amount: U256, precision: usize) -> String {
        let amount = amount.to_string().parse::<f64>().unwrap();
        let amount = amount / 10_f64.powf(self.decimals as f64);
        format!("{:.precision$}", amount)
    }

    /// Returns the ERC-20 contract address.
    ///
    /// This panics for fiat and native assets, which do not have an address.
    pub fn unwrap_address(&self) -> Address {
        match &self.identifier {
            TokenIdentifier::ERC20 { address } => *address,
            TokenIdentifier::Fiat { symbol: _ } => panic!("Fiat tokens do not have an address"),
            TokenIdentifier::Native => panic!("Native tokens do not have an address"),
        }
    }
}
