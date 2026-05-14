//! ERC-4626 Vault Quoter
//!
//! The [`ERC4626Quoter`] struct is used to quote conversion rates between a vault's shares and underlying asset.
//!
//! ```rust,ignore
//! use eth_prices::quoter::erc4626::ERC4626Quoter;
//!
//! let provider = ProviderBuilder::new().connect("https://...").await.unwrap();
//!
//! // Create a quoter for the vault
//! let quoter = ERC4626Quoter::new(vault_address, provider).await;
//!
//! // Get the token pair data (vault shares and underlying asset)
//! let (token_a, token_b) = quoter.get_tokens();
//!
//! // Get 1 of the token
//! let amount_in = token_a.nominal_amount();
//!
//! // Decide what block to query (latest in this case)
//! let block = provider.get_block_number().await.unwrap();
//!
//! // Quote the rate
//! let rate = quoter.get_rate(amount_in, RateDirection::Forward, block).await.unwrap();
//! println!("rate: {}", rate);
//! ```

use alloy::{
    primitives::{Address, U256},
    providers::DynProvider,
    sol,
};
use serde::Deserialize;

use crate::{
    EthPricesError, Result, network::Network, quoter::{Quoter, RateDirection}, asset::identity::AssetIdentifier
};

sol! {
    #[sol(rpc)]
    contract ERC4626 {
        function asset() public view returns (address);
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
    /// Vault share token metadata.
    pub vault_address: AssetIdentifier,
    /// Underlying asset metadata returned by `asset()`.
    pub token_address: AssetIdentifier,
}

impl ERC4626Quoter {
    /// Creates a quoter by loading the vault's underlying asset.
    pub async fn new(vault_address: Address, provider: &DynProvider) -> Result<Self> {
        let vault = ERC4626::new(vault_address, provider);
        let token_address = vault.asset().call().await?;
        let token_address = token_address.into();
        let vault_address = vault_address.into();
        Ok(Self {
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
        network: &Network,
    ) -> Result<U256> {
        let (_chain_id, block_number, provider) = network.as_evm().ok_or(EthPricesError::InvalidNetwork(format!("Network: {:?}", network)))?;
        let vault = ERC4626::new(
            Address::try_from(&self.vault_address)
                .map_err(|_| crate::error::EthPricesError::MissingVaultAddress)?,
            provider,
        );
        Ok(match direction {
            RateDirection::Forward => {
                vault
                    .convertToAssets(amount_in)
                    .block(alloy::eips::BlockId::Number(alloy::eips::BlockNumberOrTag::Number(*block_number)))
                    .call()
                    .await?
            }
            RateDirection::Reverse => {
                vault
                    .convertToShares(amount_in)
                    .block(alloy::eips::BlockId::Number(alloy::eips::BlockNumberOrTag::Number(*block_number)))
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
    use crate::{tests::get_test_provider, asset::Asset};

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
        let forward_rate = quoter
            .rate(token_a_amount, RateDirection::Forward, &Network::EVM(1, block, provider.clone()))
            .await
            .unwrap();

        let token_b = Asset::new(quoter.token_address.clone(), &provider)
            .await
            .unwrap();
        let token_b_amount = token_b.nominal_amount();
        let reverse_rate = quoter
            .rate(token_b_amount, RateDirection::Reverse, &Network::EVM(1, block, provider.clone()))
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
