//! Fixed rate quote sources.

use std::fmt::{self, Display};

use alloy::primitives::{U256, U512, aliases::U2048};
use serde::Deserialize;

use crate::{
    Result,
    asset::identity::AssetIdentifier,
    network::NetworkInstant,
    quoter::{Quoter, RateDirection},
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
    pub token_in: AssetIdentifier,
    pub token_in_decimals: u8,
    /// Output asset for forward quotes.
    #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
    pub token_out: AssetIdentifier,
    pub token_out_decimals: u8,
    /// Multiplier applied during forward quotes, scaled by `10^fixed_rate_decimals`.
    #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
    pub fixed_rate: U256,
    pub fixed_rate_decimals: u8,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for FixedQuoter {
    fn identity(&self) -> String {
        format!("fixed:{}:{}", self.token_in, self.token_out)
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (self.token_in.clone(), self.token_out.clone())
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        _networks: &NetworkInstant,
    ) -> Result<U256> {
        let ten = U2048::from(10);
        let token_in_scale = ten.pow(U2048::from(self.token_in_decimals));
        let token_out_scale = ten.pow(U2048::from(self.token_out_decimals));
        let rate_scale = ten.pow(U2048::from(self.fixed_rate_decimals));

        let quoted = match direction {
            RateDirection::Forward => {
                let amount_rate: U512 = amount_in.widening_mul(self.fixed_rate);
                U2048::from(amount_rate) * token_out_scale / (rate_scale * token_in_scale)
            }
            RateDirection::Reverse => {
                U2048::from(amount_in) * rate_scale * token_in_scale
                    / (U2048::from(self.fixed_rate) * token_out_scale)
            }
        };

        Ok(U256::from(quoted))
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

    use crate::{network::NetworkTime, tests::get_test_provider};

    use super::*;

    // one of the tokens is twice as big as the other
    #[tokio::test]
    async fn test_get_rate() {
        let tracker = FixedQuoter {
            token_in: AssetIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000001"),
            },
            token_in_decimals: 6,
            token_out: AssetIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000002"),
            },
            token_out_decimals: 6,
            fixed_rate: U256::from(2),
            fixed_rate_decimals: 0,
        };

        let provider = get_test_provider().await;

        let forwards = tracker
            .rate(
                U256::from(100),
                RateDirection::Forward,
                &NetworkTime::EVM(1.into(), 100, provider.clone()).instant(),
            )
            .await;
        let backwards = tracker
            .rate(
                U256::from(100),
                RateDirection::Reverse,
                &NetworkTime::EVM(1.into(), 100, provider.clone()).instant(),
            )
            .await;

        assert_eq!(forwards.unwrap(), U256::from(200));
        assert_eq!(backwards.unwrap(), U256::from(50));
    }

    // one of the tokens is 1e18, and the other is 1e6, but they are 1:1
    #[tokio::test]
    async fn test_get_rate_with_decimals() {
        let tracker = FixedQuoter {
            token_in: AssetIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000001"),
            },
            token_in_decimals: 18,
            token_out: AssetIdentifier::ERC20 {
                address: address!("0x0000000000000000000000000000000000000002"),
            },
            fixed_rate_decimals: 6,
            fixed_rate: U256::from(1000000),
            token_out_decimals: 6,
        };

        let provider = get_test_provider().await;

        let forwards = tracker
            .rate(
                U256::from(1_000_000_000_000_000_000u128),
                RateDirection::Forward,
                &NetworkTime::EVM(1.into(), 100, provider.clone()).instant(),
            )
            .await;
        let backwards = tracker
            .rate(
                U256::from(1_000_000),
                RateDirection::Reverse,
                &NetworkTime::EVM(1.into(), 100, provider.clone()).instant(),
            )
            .await;

        assert_eq!(forwards.unwrap(), U256::from(1_000_000));
        assert_eq!(
            backwards.unwrap(),
            U256::from(1_000_000_000_000_000_000u128)
        );
    }
}
