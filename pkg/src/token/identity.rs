use std::fmt::Display;

use alloy::primitives::Address;
use serde::{Deserialize, Deserializer};

/// A lightweight token identifier used by quoters and config.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenIdentifier {
    /// An ERC-20 token identified by contract address.
    ERC20 { address: Address },
    /// A fiat endpoint identified by symbol, e.g. "fiat:usd".
    Fiat { symbol: String },
    /// The native currency of a chain, e.g. "eth" on Ethereum.
    Native,
}

impl From<Address> for TokenIdentifier {
    fn from(address: Address) -> Self {
        TokenIdentifier::ERC20 { address }
    }
}

impl TryFrom<String> for TokenIdentifier {
    type Error = crate::error::EthPricesError;

    /// Parses an identifier from strings such as `0x...`, `fiat:usd`, or `native`.
    fn try_from(input: String) -> Result<Self, Self::Error> {
        if input == "native" {
            Ok(TokenIdentifier::Native)
        } else if input.starts_with("fiat:") {
            let symbol = input
                .split("fiat:")
                .nth(1)
                .ok_or(crate::error::EthPricesError::InvalidFiatSymbol)?
                .to_string();

            Ok(TokenIdentifier::Fiat { symbol })
        } else if input.starts_with("0x") {
            let address = input
                .parse::<Address>()
                .map_err(|e| crate::error::EthPricesError::InvalidAddress(e.to_string()))?;

            Ok(TokenIdentifier::ERC20 { address })
        } else {
            Err(crate::error::EthPricesError::TokenNotFound(input))
        }
    }
}

impl Display for TokenIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenIdentifier::ERC20 { address } => write!(f, "{}", address),
            TokenIdentifier::Fiat { symbol } => write!(f, "fiat:{}", symbol),
            TokenIdentifier::Native => write!(f, "native"),
        }
    }
}

impl<'de> Deserialize<'de> for TokenIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TokenIdentifier::try_from(s).map_err(serde::de::Error::custom)
    }
}
