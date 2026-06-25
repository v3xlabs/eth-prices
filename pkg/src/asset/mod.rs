//! Token metadata and identifier helpers.

use alloy::primitives::{Address, U256};

use crate::{Result, asset::erc20::ERC20, provider::RpcProvider};

pub mod erc20;
pub mod identity;

pub use identity::AssetIdentifier;

/// A resolved asset with display metadata and decimal precision.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// Canonical identifier for the asset.
    pub identifier: AssetIdentifier,
    /// Human-readable asset name.
    pub name: String,
    /// Short symbol used for display.
    pub symbol: String,
    /// Number of decimal places used by on-chain amounts.
    pub decimals: u8,
}

const FIAT_DECIMALS: u8 = 6;

impl Asset {
    /// Resolves token metadata for the provided identifier.
    ///
    /// ERC-20 metadata is loaded from chain, while fiat and native assets use local defaults.
    pub async fn new(identifier: AssetIdentifier, provider: &RpcProvider) -> Result<Self> {
        let (name, symbol, decimals) = match &identifier {
            AssetIdentifier::ERC20 { address } => {
                let erc20 = ERC20::new(*address, provider);

                (
                    erc20.name().call().await?,
                    erc20.symbol().call().await?,
                    erc20.decimals().call().await?,
                )
            }
            AssetIdentifier::Fiat { symbol } => (symbol.clone(), symbol.clone(), FIAT_DECIMALS),
            AssetIdentifier::Native => ("Native".to_string(), "ETH".to_string(), 18),
        };

        Ok(Self {
            identifier,
            name,
            symbol,
            decimals,
        })
    }

    /// Returns one nominal unit for this token in base precision.
    pub fn nominal_amount(&self) -> U256 {
        U256::from(10).pow(U256::from(self.decimals))
    }

    /// Formats a raw integer amount into a human-readable decimal string.
    pub fn format_amount(&self, amount: U256, precision: usize) -> crate::Result<String> {
        let amount = amount
            .to_string()
            .parse::<f64>()
            .map_err(|e| crate::error::EthPricesError::InvalidAssetAmount(e.to_string()))?;
        let amount = amount / 10_f64.powf(self.decimals as f64);
        Ok(format!("{:.precision$}", amount))
    }

    /// Returns the ERC-20 contract address.
    ///
    /// Returns the underlying contract address, if applicable.
    pub fn address(&self) -> Option<Address> {
        match &self.identifier {
            AssetIdentifier::ERC20 { address } => Some(*address),
            AssetIdentifier::Fiat { .. } => None,
            AssetIdentifier::Native => None,
        }
    }
}
