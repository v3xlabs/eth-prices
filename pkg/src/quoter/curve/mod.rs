//! Curve pool quote sources.

pub mod discovery;

use alloy::{
    primitives::{Address, U256},
    sol,
};

use crate::{
    EthPricesError, Result,
    asset::identity::AssetIdentifier,
    network::{NetworkId, NetworkInstant},
    quoter::{Quoter, RateDirection},
};

sol! {
    #[sol(rpc)]
    contract CurveStableSwapPool {
        function get_dy(int128 i, int128 j, uint256 dx) external view returns (uint256 dy);
    }

    #[sol(rpc)]
    contract CurveCryptoPool {
        function get_dy(uint256 i, uint256 j, uint256 dx) external view returns (uint256 dy);
    }
}

/// Distinguishes the two `get_dy` ABI variants deployed across Curve pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurvePoolKind {
    /// StableSwap pools index coins with `int128`.
    StableSwap,
    /// Crypto (twocrypto/tricrypto) pools index coins with `uint256`.
    Crypto,
}

/// Quotes spot rates from a Curve pool at a given block height.
#[derive(Debug, Clone)]
pub struct CurveQuoter {
    pub network_id: NetworkId,
    /// Pool contract address.
    pub pool_address: Address,
    /// First quoted coin.
    pub token0: Address,
    /// Second quoted coin.
    pub token1: Address,
    /// Pool coin index of `token0`.
    pub coin_index0: u8,
    /// Pool coin index of `token1`.
    pub coin_index1: u8,
    pub kind: CurvePoolKind,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for CurveQuoter {
    fn identity(&self) -> String {
        format!(
            "curve:{}:{}-{}",
            self.pool_address, self.coin_index0, self.coin_index1
        )
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (self.token0.into(), self.token1.into())
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
        let (coin_in, coin_out) = match direction {
            RateDirection::Forward => (self.coin_index0, self.coin_index1),
            RateDirection::Reverse => (self.coin_index1, self.coin_index0),
        };
        let block =
            alloy::eips::BlockId::Number(alloy::eips::BlockNumberOrTag::Number(*block_number));
        let dy = match self.kind {
            CurvePoolKind::StableSwap => {
                CurveStableSwapPool::new(self.pool_address, provider)
                    .get_dy(i128::from(coin_in), i128::from(coin_out), amount_in)
                    .block(block)
                    .call()
                    .await?
            }
            CurvePoolKind::Crypto => {
                CurveCryptoPool::new(self.pool_address, provider)
                    .get_dy(U256::from(coin_in), U256::from(coin_out), amount_in)
                    .block(block)
                    .call()
                    .await?
            }
        };
        Ok(dy)
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;

    use super::*;
    use crate::{network::NetworkTime, utils::get_test_provider};

    #[tokio::test]
    async fn test_get_rate() {
        let block = 24692474;
        let provider = get_test_provider().await;
        let quoter = CurveQuoter {
            network_id: 1.into(),
            pool_address: address!("0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7"),
            token0: address!("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
            token1: address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            coin_index0: 0,
            coin_index1: 1,
            kind: CurvePoolKind::StableSwap,
        };
        let time = NetworkTime::EVM(1.into(), block, provider.clone()).instant();

        let forward_rate = quoter
            .rate(
                U256::from(10).pow(U256::from(18)),
                RateDirection::Forward,
                &time,
            )
            .await
            .unwrap();
        let reverse_rate = quoter
            .rate(U256::from(1_000_000), RateDirection::Reverse, &time)
            .await
            .unwrap();

        assert_eq!(forward_rate, U256::from(999_840));
        assert_eq!(reverse_rate, U256::from(999_860_004_039_025_938u64));
    }
}
