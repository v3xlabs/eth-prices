use std::fmt::Display;

use alloy::primitives::Address;
use serde::{Deserialize, Deserializer};

/// A lightweight token identifier used by quoters and config.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetIdentifier {
    /// An ERC-20 token identified by contract address.
    ERC20 { address: Address },
    /// A fiat endpoint identified by symbol, e.g. "fiat:usd".
    Fiat { symbol: String },
    /// The native currency of a chain, e.g. "eth" on Ethereum.
    Native,
}

impl From<Address> for AssetIdentifier {
    fn from(address: Address) -> Self {
        AssetIdentifier::ERC20 { address }
    }
}

impl TryFrom<String> for AssetIdentifier {
    type Error = crate::error::EthPricesError;

    /// Parses an identifier from strings such as `0x...`, `fiat:usd`, or `native`.
    fn try_from(input: String) -> Result<Self, Self::Error> {
        if input == "native" {
            Ok(AssetIdentifier::Native)
        } else if input.starts_with("fiat:") {
            let symbol = input
                .split("fiat:")
                .nth(1)
                .ok_or(crate::error::EthPricesError::InvalidFiatSymbol)?
                .to_string();

            Ok(AssetIdentifier::Fiat { symbol })
        } else if input.starts_with("0x") {
            let address = input
                .parse::<Address>()
                .map_err(|e| crate::error::EthPricesError::InvalidAddress(e.to_string()))?;

            Ok(AssetIdentifier::ERC20 { address })
        } else {
            Err(crate::error::EthPricesError::TokenNotFound(input))
        }
    }
}

impl Display for AssetIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetIdentifier::ERC20 { address } => write!(f, "{}", address),
            AssetIdentifier::Fiat { symbol } => write!(f, "fiat:{}", symbol),
            AssetIdentifier::Native => write!(f, "native"),
        }
    }
}

impl<'de> Deserialize<'de> for AssetIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AssetIdentifier::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<&AssetIdentifier> for Address {
    type Error = crate::error::EthPricesError;

    fn try_from(value: &AssetIdentifier) -> Result<Self, Self::Error> {
        match value {
            AssetIdentifier::ERC20 { address } => Ok(*address),
            _ => Err(crate::error::EthPricesError::InvalidAddress(
                "invalid token identifier".to_string(),
            )),
        }
    }
}
