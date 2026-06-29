/*!
Chainlink Price Feed Quoter

The [`ChainlinkQuoter`] struct is used to quote conversion rates from a Chainlink
aggregator feed, typically a `/USD` feed (e.g. `ETH/USD`, `LINK/USD`).

```rust
use eth_prices::quoter::chainlink::ChainlinkQuoter;

let quoter = ChainlinkQuoter {
    network_id: 1.into(),
    feed_address: "0x5f4ec3df9cbd43714fe2740f5e3616155c5b8419".try_into().unwrap(),
    token_address: "fiat:usd".try_into().unwrap(),
    quote: "fiat:usd".try_into().unwrap(),
    feed_decimals: 8,
    token_decimals: 18,
    quote_decimals: 6,
};
```
*/

use std::fmt::{self, Display};

use alloy::{
    primitives::{Address, U256},
    sol,
};
use serde::Deserialize;

use crate::{
    EthPricesError, Result,
    asset::{Asset, AssetIdentifier},
    network::{NetworkId, NetworkInstant},
    provider::RpcProvider,
    quoter::{Quoter, RateDirection},
};

sol! {
    #[sol(rpc)]
    contract AggregatorV3 {
        function decimals() external view returns (uint8);
        function latestRoundData()
            external
            view
            returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
    }
}

/// Configuration for a single Chainlink feed quoter.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ChainlinkConfig {
    /// Chainlink aggregator contract address.
    pub contract: Address,
    /// Token priced by this feed.
    pub token_contract: Address,
    /// Optional destination asset; defaults to `fiat:usd`.
    #[serde(default = "default_quote")]
    pub quote: AssetIdentifier,
}

fn default_quote() -> AssetIdentifier {
    AssetIdentifier::Fiat {
        symbol: "usd".to_string(),
    }
}

/// Quotes conversions between a token and a Chainlink feed quote asset.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainlinkQuoter {
    pub network_id: NetworkId,
    pub feed_address: Address,
    pub token_address: AssetIdentifier,
    pub quote: AssetIdentifier,
    pub feed_decimals: u8,
    pub token_decimals: u8,
    pub quote_decimals: u8,
}

impl ChainlinkQuoter {
    /// Creates a quoter by loading feed and token metadata from chain.
    pub async fn new(
        feed_address: Address,
        token_address: Address,
        quote: AssetIdentifier,
        provider: &RpcProvider,
    ) -> Result<Self> {
        let network_id = NetworkId::from_provider(provider).await?;
        let feed = AggregatorV3::new(feed_address, provider);
        let feed_decimals = feed.decimals().call().await?;

        let token_asset = Asset::new(token_address.into(), provider).await?;
        let token_decimals = token_asset.decimals;
        let quote_decimals = Self::resolve_quote_decimals(&quote, provider).await?;

        Ok(Self {
            network_id,
            feed_address,
            token_address: token_address.into(),
            quote,
            feed_decimals,
            token_decimals,
            quote_decimals,
        })
    }

    async fn resolve_quote_decimals(quote: &AssetIdentifier, provider: &RpcProvider) -> Result<u8> {
        match quote {
            AssetIdentifier::Fiat { .. } => Ok(6),
            AssetIdentifier::ERC20 { .. } => {
                let asset = Asset::new(quote.clone(), provider).await?;
                Ok(asset.decimals)
            }
            AssetIdentifier::Native => Ok(18),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for ChainlinkQuoter {
    fn identity(&self) -> String {
        format!(
            "chainlink:{}:{}:{}",
            self.feed_address, self.token_address, self.quote
        )
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (self.token_address.clone(), self.quote.clone())
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        networks: &NetworkInstant,
    ) -> Result<U256> {
        let network =
            networks
                .get(&self.network_id.clone().into())
                .ok_or(EthPricesError::InvalidNetwork(format!(
                    "Network: {:?}",
                    self.network_id
                )))?;
        let (_chain_id, block_number, provider) =
            network
                .as_evm()
                .ok_or(EthPricesError::InvalidNetwork(format!(
                    "Network: {:?}",
                    network
                )))?;

        let feed = AggregatorV3::new(self.feed_address, provider);
        let round = feed
            .latestRoundData()
            .block(alloy::eips::BlockId::Number(
                alloy::eips::BlockNumberOrTag::Number(*block_number),
            ))
            .call()
            .await?;

        if round.answer < alloy::primitives::I256::ZERO {
            return Err(EthPricesError::InvalidAssetAmount(
                "Chainlink feed returned a negative answer".to_string(),
            ));
        }

        // `answer` is the price of 1 base unit of the token in quote terms
        // scaled by 10^feed_decimals. For /USD feeds this is USD per token.
        let rate = U256::try_from(round.answer)
            .map_err(|_| EthPricesError::InvalidAssetAmount("negative rate".to_string()))?;

        // Token input units are in 10^token_decimals; feed rate is in quote units
        // per token. Rebase the output amount to the quote asset's decimals.
        let ten = alloy::primitives::aliases::U2048::from(10);
        let token_scale = ten.pow(alloy::primitives::aliases::U2048::from(self.token_decimals));
        let rate_scale = ten.pow(alloy::primitives::aliases::U2048::from(self.feed_decimals));
        let quote_scale = ten.pow(alloy::primitives::aliases::U2048::from(self.quote_decimals));

        let quoted = match direction {
            RateDirection::Forward => {
                let amount_rate: alloy::primitives::U512 = amount_in.widening_mul(rate);
                alloy::primitives::aliases::U2048::from(amount_rate) * quote_scale
                    / (rate_scale * token_scale)
            }
            RateDirection::Reverse => {
                alloy::primitives::aliases::U2048::from(amount_in) * rate_scale * token_scale
                    / (alloy::primitives::aliases::U2048::from(rate) * quote_scale)
            }
        };

        Ok(U256::from(quoted))
    }
}

impl Display for ChainlinkQuoter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chainlink:{}", self.feed_address)
    }
}

#[cfg(test)]
mod tests {
    use alloy::{primitives::address, providers::Provider};

    use super::*;
    use crate::{network::NetworkTime, utils::get_test_provider};

    // LINK/USD feed at the latest block
    #[tokio::test]
    async fn test_get_rate_link_usd() {
        let link_address = address!("0x514910771AF9Ca656af840dff83E8264EcF986CA");
        let link_usd_feed = address!("0x2c1d072E956AFFC0D435Cb7AC38EF18d24d9127c");

        let provider = get_test_provider().await;
        let block = provider.get_block_number().await.unwrap();
        let quote = AssetIdentifier::Fiat {
            symbol: "usd".to_string(),
        };
        let quoter = ChainlinkQuoter::new(link_usd_feed, link_address, quote, &provider)
            .await
            .unwrap();

        assert_eq!(quoter.token_decimals, 18);
        assert_eq!(quoter.feed_decimals, 8);

        let link_amount = U256::from(10).pow(U256::from(18));
        let time = NetworkTime::EVM(1.into(), block, provider.clone()).instant();
        let forward_rate = quoter
            .rate(link_amount, RateDirection::Forward, &time)
            .await
            .unwrap();

        // LINK at time of writing ~ $12-15; fiat output is in 6 decimals
        assert!(
            forward_rate > U256::from(5_000_000) && forward_rate < U256::from(50_000_000),
            "expected LINK USD price in range, got {forward_rate}"
        );

        // Reverse the quoted USD amount should give back ~1 LINK
        let reverse_rate = quoter
            .rate(forward_rate, RateDirection::Reverse, &time)
            .await
            .unwrap();

        let diff = if reverse_rate > link_amount {
            reverse_rate - link_amount
        } else {
            link_amount - reverse_rate
        };
        assert!(
            diff <= U256::from(10).pow(U256::from(15)),
            "reverse should be within 0.001 LINK of 1 LINK; got {reverse_rate}"
        );

        println!("forward_rate: {} (USD with 6 decimals)", forward_rate);
        println!("reverse_rate: {} (LINK)", reverse_rate);
    }
}
