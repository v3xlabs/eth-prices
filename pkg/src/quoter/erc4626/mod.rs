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
//! let amount_in = token_a.nominal_amount().await;
//!
//! // Decide what block to query (latest in this case)
//! let block = provider.get_block_number().await.unwrap();
//!
//! // Quote the rate
//! let rate = quoter.get_rate(amount_in, RateDirection::Forward, block).await.unwrap();
//! println!("rate: {}", rate);
//! ```

use alloy::primitives::{Address, BlockNumber, U256};
use alloy::providers::DynProvider;

use crate::Result;
use alloy::sol;
use serde::Deserialize;

use crate::quoter::{Quoter, RateDirection};
use crate::token::Token;
use crate::token::identity::TokenIdentifier;

sol! {
    #[sol(rpc)]
    contract ERC4626 {
        function asset() public view returns (address);
        function convertToAssets(uint256 shares) public view returns (uint256);
        function convertToShares(uint256 assets) public view returns (uint256);
    }
}

/// Configuration for a single ERC-4626 vault quoter.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ERC4626Config {
    /// Vault contract address.
    pub vault_address: Address,
}

/// Quotes conversions between an ERC-4626 vault share token and its underlying asset.
#[derive(Debug, Clone)]
pub struct ERC4626Quoter {
    /// Vault share token metadata.
    pub vault_address: Token,
    /// Underlying asset metadata returned by `asset()`.
    pub token_address: Token,
    /// Provider used to fetch historical conversions.
    pub provider: DynProvider,
}

impl ERC4626Quoter {
    /// Creates a quoter by loading the vault's underlying asset.
    pub async fn new(vault_address: Address, provider: &DynProvider) -> Result<Self> {
        let vault = ERC4626::new(vault_address, provider);
        let token_address = vault.asset().call().await?;
        let token_address = Token::new(token_address.into(), provider).await?;
        let vault_address = Token::new(vault_address.into(), provider).await?;
        Ok(Self {
            vault_address,
            token_address,
            provider: provider.clone(),
        })
    }
}

impl Quoter for ERC4626Quoter {
    fn id(&self) -> String {
        format!("erc4626:{}", self.vault_address.identifier)
    }

    fn tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        (
            self.vault_address.identifier.clone(),
            self.token_address.identifier.clone(),
        )
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> Result<U256> {
        let vault = ERC4626::new(
            self.vault_address
                .address()
                .ok_or(crate::error::EthPricesError::MissingVaultAddress)?,
            &self.provider,
        );
        Ok(match direction {
            RateDirection::Forward => {
                vault
                    .convertToAssets(amount_in)
                    .block(block.into())
                    .call()
                    .await?
            }
            RateDirection::Reverse => {
                vault
                    .convertToShares(amount_in)
                    .block(block.into())
                    .call()
                    .await?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::get_test_provider, token::Token};
    use alloy::primitives::address;

    #[tokio::test]
    async fn test_get_rate() {
        let block = 24692474;
        let vault_address = address!("0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1");

        let provider = get_test_provider().await;
        let quoter = ERC4626Quoter::new(vault_address, &provider).await.unwrap();

        let token_a = Token::new(quoter.vault_address.identifier.clone(), &provider)
            .await
            .unwrap();
        let token_a_amount = token_a.nominal_amount().await;
        let forward_rate = quoter
            .rate(token_a_amount, RateDirection::Forward, block)
            .await
            .unwrap();

        let token_b = Token::new(quoter.token_address.identifier.clone(), &provider)
            .await
            .unwrap();
        let token_b_amount = token_b.nominal_amount().await;
        let reverse_rate = quoter
            .rate(token_b_amount, RateDirection::Reverse, block)
            .await
            .unwrap();

        assert_eq!(forward_rate, U256::from(1020816));
        assert_eq!(reverse_rate, U256::from(979608427435069667u64));

        let precision = 4;
        println!(
            "forward_rate: {:?} = {:?}",
            token_a.format_amount(token_a_amount, precision).await,
            token_b.format_amount(forward_rate, precision).await
        );
        println!(
            "reverse_rate: {:?} = {:?}",
            token_b.format_amount(token_b_amount, precision).await,
            token_a.format_amount(reverse_rate, precision).await
        );
    }
}
