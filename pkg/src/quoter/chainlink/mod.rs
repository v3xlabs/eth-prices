/*!
Chainlink Price Feed Quoter

Quotes conversion rates from [Chainlink Data Feeds](https://docs.chain.link/data-feeds).

Each feed reports a single numeric answer — the price of one unit of the base
asset in quote terms — scaled by the feed's `decimals()`.  The quoter applies
the feed's raw answer directly; it never makes on-chain calls to any token
contract, so it works with *any* feed: crypto pairs (ETH/USD, LINK/ETH),
equity indices, commodities, etc.  The quote asset is specified when creating
the quoter.

# Configuration

The [`ChainlinkConfig`] struct maps a feed to a `token` and `quote` identifier
with optional decimal overrides:

```json
{
    "contract": "0x5f4ec3df9cbd43714fe2740f5e3616155c5b8419",
    "token": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "token_decimals": 18,
    "quote": "fiat:usd",
    "quote_decimals": 6
}
```

Non-EVM assets (stocks, commodities) use the `token` field as an arbitrary
string — no on-chain metadata is required:

```json
{
    "contract": "0x139c8512cde1778e9b9a8e721ce1aebd4dd43587",
    "token": "aapl",
    "quote": "fiat:usd"
}
```

When `token_decimals` / `quote_decimals` are omitted they are inferred from the
identifier type (ERC-20 → 18, Native → 18, Fiat → 6, Custom → 0).
*/

use std::fmt::{self, Display};

use alloy::{
    primitives::{Address, U256},
    sol,
};
use serde::Deserialize;

use crate::{
    EthPricesError, Result,
    asset::AssetIdentifier,
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

fn infer_decimals(id: &AssetIdentifier) -> u8 {
    match id {
        AssetIdentifier::ERC20 { .. } => 18,
        AssetIdentifier::Native => 18,
        AssetIdentifier::Fiat { .. } => 6,
        AssetIdentifier::Custom(_) => 0,
    }
}

/// Configuration for a single Chainlink feed quoter.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ChainlinkConfig {
    /// Chainlink aggregator contract address.
    pub contract: Address,
    /// The base asset priced by this feed (e.g. `0x...`, `fiat:usd`, `native`, or a custom string like `"aapl"`).
    pub token: AssetIdentifier,
    /// Override for the base asset's decimal precision. Inferred from `token` when absent.
    #[serde(default)]
    pub token_decimals: Option<u8>,
    /// The quote asset (defaults to `fiat:usd`).
    #[serde(default = "default_quote")]
    pub quote: AssetIdentifier,
    /// Override for the quote asset's decimal precision. Inferred from `quote` when absent.
    #[serde(default)]
    pub quote_decimals: Option<u8>,
}

fn default_quote() -> AssetIdentifier {
    AssetIdentifier::Fiat {
        symbol: "usd".to_string(),
    }
}

/// Quotes conversions between a token and a Chainlink feed quote asset.
///
/// No on-chain token metadata is queried — the feed's own `answer` and
/// `decimals()` are the only on-chain data used.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainlinkQuoter {
    pub network_id: NetworkId,
    pub feed_address: Address,
    pub token: AssetIdentifier,
    pub quote: AssetIdentifier,
    pub feed_decimals: u8,
    pub token_decimals: u8,
    pub quote_decimals: u8,
}

impl ChainlinkQuoter {
    /// Creates a quoter that reads the feed on chain and uses the provided
    /// decimal values (or sensible inference).
    pub async fn new(
        feed_address: Address,
        token: AssetIdentifier,
        token_decimals: Option<u8>,
        quote: AssetIdentifier,
        quote_decimals: Option<u8>,
        provider: &RpcProvider,
    ) -> Result<Self> {
        let network_id = NetworkId::from_provider(provider).await?;
        let feed = AggregatorV3::new(feed_address, provider);
        let feed_decimals = feed.decimals().call().await?;

        let tk_decimals = token_decimals.unwrap_or_else(|| infer_decimals(&token));
        let qt_decimals = quote_decimals.unwrap_or_else(|| infer_decimals(&quote));

        Ok(Self {
            network_id,
            feed_address,
            token,
            quote,
            feed_decimals,
            token_decimals: tk_decimals,
            quote_decimals: qt_decimals,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for ChainlinkQuoter {
    fn identity(&self) -> String {
        format!(
            "chainlink:{}:{}:{}",
            self.feed_address, self.token, self.quote
        )
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (self.token.clone(), self.quote.clone())
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        networks: &NetworkInstant,
    ) -> Result<U256> {
        let network = networks
            .get(&self.network_id.clone().into())
            .ok_or(EthPricesError::InvalidNetwork(format!(
                "Network: {:?}",
                self.network_id
            )))?;
        let (_chain_id, block_number, provider) = network
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

        let rate = U256::try_from(round.answer)
            .map_err(|_| EthPricesError::InvalidAssetAmount("negative rate".to_string()))?;

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

    #[tokio::test]
    async fn test_get_rate_link_usd() {
        let link_feed = address!("0x2c1d072E956AFFC0D435Cb7AC38EF18d24d9127c");
        let link_token = AssetIdentifier::ERC20 {
            address: address!("0x514910771AF9Ca656af840dff83E8264EcF986CA"),
        };
        let quote = AssetIdentifier::Fiat {
            symbol: "usd".to_string(),
        };

        let provider = get_test_provider().await;
        let block = provider.get_block_number().await.unwrap();
        let quoter = ChainlinkQuoter::new(
            link_feed,
            link_token.clone(),
            Some(18),
            quote,
            Some(6),
            &provider,
        )
        .await
        .unwrap();

        assert_eq!(quoter.feed_decimals, 8);

        let link_amount = U256::from(10).pow(U256::from(18));
        let time = NetworkTime::EVM(1.into(), block, provider.clone()).instant();
        let forward_rate = quoter
            .rate(link_amount, RateDirection::Forward, &time)
            .await
            .unwrap();

        assert!(
            forward_rate > U256::from(5_000_000) && forward_rate < U256::from(50_000_000),
            "expected LINK ~5-50 USD (6 decimals), got {forward_rate}"
        );

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

    #[tokio::test]
    async fn test_get_rate_custom_token() {
        // SOL/USD feed — SOL is a non-EVM token, proving custom identifiers work
        let feed = address!("0x4ffC43a60e009B551865A93d232E33Fce9f01507");
        let token = AssetIdentifier::Custom("sol".to_string());
        let quote = AssetIdentifier::Fiat {
            symbol: "usd".to_string(),
        };

        let provider = get_test_provider().await;
        let block = provider.get_block_number().await.unwrap();
        let quoter = ChainlinkQuoter::new(feed, token.clone(), Some(9), quote, Some(6), &provider)
            .await
            .unwrap();

        assert_eq!(quoter.feed_decimals, 8);
        assert_eq!(quoter.token_decimals, 9);

        // 1 SOL (9 decimal places)
        let one_sol = U256::from(10).pow(U256::from(9));
        let time = NetworkTime::EVM(1.into(), block, provider.clone()).instant();
        let forward_rate = quoter
            .rate(one_sol, RateDirection::Forward, &time)
            .await
            .unwrap();

        // SOL ~$100-300 USD → output is in 6 decimal fiat units
        assert!(
            forward_rate > U256::from(50_000_000) && forward_rate < U256::from(500_000_000),
            "expected SOL ~50-500 USD (6 decimals), got {forward_rate}"
        );

        let reverse_rate = quoter
            .rate(forward_rate, RateDirection::Reverse, &time)
            .await
            .unwrap();

        let diff = if reverse_rate > one_sol {
            reverse_rate - one_sol
        } else {
            one_sol - reverse_rate
        };
        assert!(
            diff <= U256::from(1000),
            "reverse should be within 0.000001 SOL of 1 SOL; got {reverse_rate}"
        );

        println!("forward_rate: {} (USD with 6 decimals)", forward_rate);
        println!("reverse_rate: {} (SOL)", reverse_rate);
    }
}
