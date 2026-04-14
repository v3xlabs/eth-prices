//! Fixed rate quote sources.

use std::fmt::{self, Display};

use crate::Result;
use alloy::primitives::{BlockNumber, U256};
use serde::Deserialize;

use crate::{
    quoter::{Quoter, RateDirection},
    token::identity::TokenIdentifier,
};

/// A static conversion rate between two assets.
///
/// This is mainly useful for synthetic edges such as fiat pegs or test fixtures.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    serde(rename_all = "camelCase"),
    tsify(from_wasm_abi)
)]
pub struct FixedQuoter {
    /// Input asset for forward quotes.
    #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
    pub token_in: TokenIdentifier,
    /// Output asset for forward quotes.
    #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
    pub token_out: TokenIdentifier,
    /// Multiplier applied during forward quotes.
    pub fixed_rate: f64,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for FixedQuoter {
    fn identity(&self) -> String {
        format!("fixed:{}:{}", self.token_in, self.token_out)
    }

    fn tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        (self.token_in.clone(), self.token_out.clone())
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        _block: BlockNumber,
    ) -> Result<U256> {
        match direction {
            // TODO: Check this math
            RateDirection::Forward => Ok(U256::from(
                self.fixed_rate
                    * amount_in.to_string().parse::<f64>().map_err(|e| {
                        crate::error::EthPricesError::InvalidTokenAmount(e.to_string())
                    })?,
            )),
            RateDirection::Reverse => Ok(U256::from(
                1.0 / self.fixed_rate
                    * amount_in.to_string().parse::<f64>().map_err(|e| {
                        crate::error::EthPricesError::InvalidTokenAmount(e.to_string())
                    })?,
            )),
        }
    }
}

impl Display for FixedQuoter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fixed:{}:{}", self.token_in, self.token_out)
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;

    use super::*;

    #[tokio::test]
    async fn test_get_rate() {
        let tracker = FixedQuoter {
            token_in: TokenIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000001"),
            },
            token_out: TokenIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000002"),
            },
            fixed_rate: 2.0,
        };

        let forwards = tracker
            .rate(U256::from(100), RateDirection::Forward, 100)
            .await;
        let backwards = tracker
            .rate(U256::from(100), RateDirection::Reverse, 100)
            .await;

        assert_eq!(forwards.unwrap(), U256::from(200));
        assert_eq!(backwards.unwrap(), U256::from(50));
    }
}
