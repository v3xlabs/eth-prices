/*!
ERC-4626 Vault Quoter

The [`ERC4626Quoter`] struct is used to quote conversion rates between a vault's shares and underlying asset.

```rust
use eth_prices::{quoter::erc4626::ERC4626Quoter, quoter::Quoter, asset::{Asset, AssetIdentifier}, network::NetworkTime, quoter::RateDirection};
use alloy::{providers::{ProviderBuilder, Provider}, primitives::address};

#[tokio::main]
pub async fn main() {
    let provider = ProviderBuilder::new().connect("https://ethereum.reth.rs/rpc").await.unwrap().erased();
    let vault_address = address!("0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1");
    // Create a quoter for the vault
    let quoter = ERC4626Quoter::new(vault_address, &provider).await.unwrap();
    // Get the token pair data (vault shares and underlying asset)
    let (token_a, token_b) = quoter.tokens();
    let token_a = Asset::new(token_a, &provider).await.unwrap();
    let token_b = Asset::new(token_b, &provider).await.unwrap();
    // Get 1 of the token
    let amount_in = token_a.nominal_amount();
    // Decide what block to query (latest in this case)
    let block = provider.get_block_number().await.unwrap();
    let network = NetworkTime::EVM(1.into(), block, provider.clone()).instant();
    // Quote the rate
    let rate = quoter.rate(amount_in, RateDirection::Forward, &network).await.unwrap();
    println!("rate: {}", rate);
}
```

## Discovery

The [`ERC4626Quoter`] supports discovery and is supported by the [`crate::router::AutoRouter`] when enabled.
Given the vault address, it queries `asset()` to get the underlying asset address and constructs a quoter.
*/

use alloy::{
    primitives::{Address, U256},
    sol,
};
use serde::Deserialize;

use crate::{
    EthPricesError, Result,
    asset::identity::AssetIdentifier,
    network::{NetworkId, NetworkInstant},
    provider::RpcProvider,
    quoter::{Quoter, RateDirection},
};

sol! {
    #[sol(rpc)]
    contract ERC4626 {
        function asset() public view returns (address);
        function totalAssets() external view returns (uint256);
        function convertToAssets(uint256 shares) public view returns (uint256);
        function convertToShares(uint256 assets) public view returns (uint256);
    }
}

/// Configuration for a single ERC-4626 vault quoter.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ERC4626Config {
    /// Vault contract address.
    pub vault_address: Address,
}

/// Quotes conversions between an ERC-4626 vault share token and its underlying asset.
#[derive(Debug, Clone)]
pub struct ERC4626Quoter {
    pub network_id: NetworkId,
    /// Vault share token metadata.
    pub vault_address: AssetIdentifier,
    /// Underlying asset metadata returned by `asset()`.
    pub token_address: AssetIdentifier,
}

impl ERC4626Quoter {
    /// Creates a quoter by loading the vault's underlying asset.
    pub async fn new(vault_address: Address, provider: &RpcProvider) -> Result<Self> {
        let network_id = NetworkId::from_provider(provider).await?;
        let vault = ERC4626::new(vault_address, provider);
        let token_address = vault.asset().call().await?;
        let token_address = token_address.into();
        let vault_address = vault_address.into();
        Ok(Self {
            network_id,
            vault_address,
            token_address,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for ERC4626Quoter {
    fn identity(&self) -> String {
        format!("erc4626:{}", self.vault_address)
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (self.vault_address.clone(), self.token_address.clone())
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
        let vault = ERC4626::new(
            Address::try_from(&self.vault_address)
                .map_err(|_| crate::error::EthPricesError::MissingVaultAddress)?,
            provider,
        );
        Ok(match direction {
            RateDirection::Forward => {
                vault
                    .convertToAssets(amount_in)
                    .block(alloy::eips::BlockId::Number(
                        alloy::eips::BlockNumberOrTag::Number(*block_number),
                    ))
                    .call()
                    .await?
            }
            RateDirection::Reverse => {
                vault
                    .convertToShares(amount_in)
                    .block(alloy::eips::BlockId::Number(
                        alloy::eips::BlockNumberOrTag::Number(*block_number),
                    ))
                    .call()
                    .await?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;

    use super::*;
    use crate::{asset::Asset, network::NetworkTime, utils::get_test_provider};

    #[tokio::test]
    async fn test_get_rate() {
        let block = 25000000;
        let vault_address = address!("0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1");

        let provider = get_test_provider().await;
        let quoter = ERC4626Quoter::new(vault_address, &provider).await.unwrap();

        let token_a = Asset::new(quoter.vault_address.clone(), &provider)
            .await
            .unwrap();
        let token_a_amount = token_a.nominal_amount();
        let time = NetworkTime::EVM(1.into(), block, provider.clone()).instant();
        let forward_rate = quoter
            .rate(token_a_amount, RateDirection::Forward, &time)
            .await
            .unwrap();

        let token_b = Asset::new(quoter.token_address.clone(), &provider)
            .await
            .unwrap();
        let token_b_amount = token_b.nominal_amount();
        let reverse_rate = quoter
            .rate(token_b_amount, RateDirection::Reverse, &time)
            .await
            .unwrap();

        assert_eq!(forward_rate, U256::from(1023479));
        assert_eq!(reverse_rate, U256::from(977058994841501187u64));

        let precision = 4;
        println!(
            "forward_rate: {:?} = {:?}",
            token_a.format_amount(token_a_amount, precision),
            token_b.format_amount(forward_rate, precision)
        );
        println!(
            "reverse_rate: {:?} = {:?}",
            token_b.format_amount(token_b_amount, precision),
            token_a.format_amount(reverse_rate, precision)
        );
    }
}
