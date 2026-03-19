//! Fixed rate quote sources.

use alloy::primitives::{BlockNumber, U256};
use anyhow::Result;
use serde::Deserialize;

use crate::{
    quoters::{Quoter, RateDirection},
    token::identity::TokenIdentifier,
};

/// A static conversion rate between two assets.
///
/// This is mainly useful for synthetic edges such as fiat pegs or test fixtures.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct FixedTracker {
    /// Input asset for forward quotes.
    pub token_in: TokenIdentifier,
    /// Output asset for forward quotes.
    pub token_out: TokenIdentifier,
    /// Multiplier applied during forward quotes.
    pub fixed_rate: f64,
}

impl Quoter for FixedTracker {
    fn get_slug(&self) -> String {
        format!("fixed:{}:{}", self.token_in, self.token_out)
    }

    fn get_tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        (self.token_in.clone(), self.token_out.clone())
    }

    async fn get_rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        _block: BlockNumber,
    ) -> Result<U256> {
        match direction {
            RateDirection::Forward => Ok(U256::from(self.fixed_rate * amount_in.to_string().parse::<f64>()?)),
            RateDirection::Reverse => Ok(U256::from(1.0 / self.fixed_rate * amount_in.to_string().parse::<f64>()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;

    use super::*;

    #[tokio::test]
    async fn test_get_rate() {
        let tracker = FixedTracker {
            token_in: TokenIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000001"),
            },
            token_out: TokenIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000002"),
            },
            fixed_rate: 2.0,
        };

        let forwards = tracker
            .get_rate(U256::from(100), RateDirection::Forward, 100)
            .await;
        let backwards = tracker
            .get_rate(U256::from(100), RateDirection::Reverse, 100)
            .await;

        assert_eq!(forwards.unwrap(), U256::from(200));
        assert_eq!(backwards.unwrap(), U256::from(50));
    }
}
