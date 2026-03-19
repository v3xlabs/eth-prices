//! ERC-4626 vault quote sources.
//!
//! These quoters map between vault shares and the underlying asset using the vault's
//! conversion functions at a specific block.

use alloy::primitives::{Address, BlockNumber, U256};
use alloy::providers::DynProvider;

use alloy::sol;
use serde::Deserialize;

use crate::quoters::{Quoter, RateDirection};
use crate::token::local::LocalTokenOrFiat;

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
    /// Vault contract address.
    pub vault_address: Address,
    /// Underlying asset returned by `asset()`.
    pub token_address: Address,
    /// Provider used to fetch historical conversions.
    pub provider: DynProvider,
}

impl ERC4626Quoter {
    /// Creates a quoter by loading the vault's underlying asset.
    pub async fn new(vault_address: Address, provider: &DynProvider) -> Self {
        let vault = ERC4626::new(vault_address, provider);
        let token_address = vault.asset().call().await.unwrap();
        Self {
            vault_address,
            token_address,
            provider: provider.clone(),
        }
    }
}

impl Quoter for ERC4626Quoter {
    fn get_slug(&self) -> String {
        format!("erc4626:{}:{}", self.vault_address, self.token_address)
    }

    fn get_tokens(&self) -> (LocalTokenOrFiat, LocalTokenOrFiat) {
        (self.vault_address.into(), self.token_address.into())
    }

    async fn get_rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> U256 {
        let vault = ERC4626::new(self.vault_address, &self.provider);
        match direction {
            RateDirection::Forward => {
                let rate = vault
                    .convertToAssets(amount_in)
                    .block(block.into())
                    .call()
                    .await
                    .unwrap();
                rate
            }
            RateDirection::Reverse => {
                let rate = vault
                    .convertToShares(amount_in)
                    .block(block.into())
                    .call()
                    .await
                    .unwrap();
                rate
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::get_test_provider, token::erc20::ERC20Token};
    use alloy::primitives::address;

    #[tokio::test]
    async fn test_get_rate() {
        let block = 24692474;
        let vault_address = address!("0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1");

        let provider = get_test_provider().await;
        let quoter = ERC4626Quoter::new(vault_address, provider).await;

        let token_a = ERC20Token::new(quoter.vault_address, provider).await;
        let token_a_amount = token_a.nominal_amount().await;
        let forward_rate = quoter
            .get_rate(token_a_amount, RateDirection::Forward, block)
            .await;

        let token_b = ERC20Token::new(quoter.token_address, provider).await;
        let token_b_amount = token_b.nominal_amount().await;
        let reverse_rate = quoter
            .get_rate(token_b_amount, RateDirection::Reverse, block)
            .await;

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
