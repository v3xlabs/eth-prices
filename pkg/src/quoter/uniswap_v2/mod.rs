//! Uniswap v2 quote sources.

pub mod deployments;
pub mod discovery;
pub mod pair;

use alloy::primitives::{Address, U256, U512, address};
use pair::UniswapV2Pair::{self, UniswapV2PairInstance};
use serde::Deserialize;
use tracing::info;

use crate::{
    EthPricesError, Result,
    asset::identity::AssetIdentifier,
    network::{NetworkId, NetworkInstant},
    provider::RpcProvider,
    quoter::{Quoter, RateDirection},
};

/// Configuration for a set of Uniswap v2 pools on a single chain.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UniswapV2Config {
    /// Factory contract used when resolving pools from token pairs.
    pub factory_address: Address,
    /// Pools to load as quoters.
    pub pairs: Vec<UniswapV2Selector>,
}

/// Selects a Uniswap v2 pool either by tokens or by pair address.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    serde(rename_all = "camelCase"),
    tsify(from_wasm_abi)
)]
pub enum UniswapV2Selector {
    /// Resolve the pair address from token addresses.
    ByTokens {
        #[cfg_attr(target_arch = "wasm32", serde(rename = "tokenIn"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        token_in: Address,
        #[cfg_attr(target_arch = "wasm32", serde(rename = "tokenOut"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        token_out: Address,
        #[serde(default)]
        #[cfg_attr(target_arch = "wasm32", serde(rename = "factory"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string | undefined"))]
        factory: Option<Address>,
    },
    /// Use an already-known pair contract address.
    Pair {
        #[cfg_attr(target_arch = "wasm32", serde(rename = "pairAddress"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        pair_address: Address,
    },
}

/// Quotes spot rates from a Uniswap v2 pair contract at a given block height.
#[derive(Debug, Clone)]
pub struct UniswapV2Quoter {
    pub network_id: NetworkId,
    /// Pair contract address.
    pub pair_address: Address,
    /// First token in canonical pair order.
    pub token0: Address,
    /// Second token in canonical pair order.
    pub token1: Address,
}

impl UniswapV2Quoter {
    /// Builds a quoter from an instantiated pair contract.
    pub async fn from_contract(contract: UniswapV2PairInstance<&RpcProvider>) -> Result<Self> {
        let network_id = NetworkId::from_provider(contract.provider()).await?;
        let pair_address = *contract.address();
        let token0 = contract.token0().call().await?;
        let token1 = contract.token1().call().await?;

        Ok(Self {
            network_id,
            pair_address,
            token0,
            token1,
        })
    }
}

impl UniswapV2Quoter {
    /// Builds a quoter from a selector.
    ///
    /// When a token pair is provided, the configured factory is used to discover the pair address.
    pub async fn from_selector(
        provider: &RpcProvider,
        selector: UniswapV2Selector,
    ) -> Result<Self> {
        let network_id = NetworkId::from_provider(provider).await?;

        match selector {
            UniswapV2Selector::ByTokens {
                token_in,
                token_out,
                factory,
            } => {
                let factory_address =
                    factory.unwrap_or(address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"));
                let pair_address =
                    discovery::fetch_pair(provider, factory_address, token_in, token_out).await?;

                let (token0, token1) = if token_in < token_out {
                    (token_in, token_out)
                } else {
                    (token_out, token_in)
                };

                Ok(Self {
                    network_id,
                    pair_address,
                    token0,
                    token1,
                })
            }
            UniswapV2Selector::Pair { pair_address } => {
                let pair = UniswapV2Pair::new(pair_address, provider);

                Self::from_contract(pair).await
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for UniswapV2Quoter {
    fn identity(&self) -> String {
        format!("uniswap_v2:{}", self.pair_address)
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (
            AssetIdentifier::ERC20 {
                address: self.token0,
            },
            AssetIdentifier::ERC20 {
                address: self.token1,
            },
        )
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
        let pair = UniswapV2Pair::new(self.pair_address, provider);
        let reserves = pair
            .getReserves()
            .call()
            .block(alloy::eips::BlockId::Number(
                alloy::eips::BlockNumberOrTag::Number(*block_number),
            ))
            .await?;
        let reserve0 = U512::from(reserves.reserve0);
        let reserve1 = U512::from(reserves.reserve1);
        let amount_in = U512::from(amount_in);
        let scale = U512::from(10).pow(U512::from(8));
        match direction {
            RateDirection::Forward => {
                info!("amount_in: {:?}", amount_in);
                info!("reserve0: {:?}", reserve0);
                info!("reserve1: {:?}", reserve1);
                info!("scale: {:?}", scale);

                let numerator = amount_in * reserve1;
                let denominator = reserve0;

                let amount_out = numerator / denominator;

                Ok(U256::from(amount_out))
            }
            RateDirection::Reverse => {
                let numerator = amount_in * reserve0;
                let denominator = reserve1;

                let amount_out = numerator / denominator;

                Ok(U256::from(amount_out))
            }
        }
    }
}
