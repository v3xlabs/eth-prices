//! Uniswap v2 quote sources.

pub mod factory;
pub mod pair;

use crate::Result;
use alloy::{
    primitives::{Address, BlockNumber, U256, U512, address},
    providers::DynProvider,
};
use pair::UniswapV2Pair::{self, UniswapV2PairInstance};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    quoter::{Quoter, RateDirection},
    token::identity::TokenIdentifier,
};

/// Configuration for a set of Uniswap v2 pools on a single chain.
#[derive(Debug, Deserialize, PartialEq)]
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
    /// Pair contract address.
    pub pair_address: Address,
    /// First token in canonical pair order.
    pub token0: Address,
    /// Second token in canonical pair order.
    pub token1: Address,
    /// Provider used to fetch historical reserves.
    pub provider: DynProvider,
}

impl UniswapV2Quoter {
    /// Builds a quoter from an instantiated pair contract.
    pub async fn from_contract(
        contract: UniswapV2PairInstance<&DynProvider>,
        provider: &DynProvider,
    ) -> Result<Self> {
        let pair_address = *contract.address();
        let token0 = contract.token0().call().await?;
        let token1 = contract.token1().call().await?;

        Ok(Self {
            pair_address,
            token0,
            token1,
            provider: provider.clone(),
        })
    }
}

impl UniswapV2Quoter {
    /// Builds a quoter from a selector.
    ///
    /// When a token pair is provided, the configured factory is used to discover the pair address.
    pub async fn from_selector(
        provider: &DynProvider,
        selector: UniswapV2Selector,
    ) -> Result<Self> {
        let factory_address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");

        match selector {
            UniswapV2Selector::ByTokens {
                token_in,
                token_out,
            } => {
                let pair_address =
                    factory::fetch_pair(provider, factory_address, token_in, token_out).await?;

                let (token0, token1) = if token_in < token_out {
                    (token_in, token_out)
                } else {
                    (token_out, token_in)
                };

                Ok(Self {
                    pair_address,
                    token0,
                    token1,
                    provider: provider.clone(),
                })
            }
            UniswapV2Selector::Pair { pair_address } => {
                let pair = UniswapV2Pair::new(pair_address, provider);

                Self::from_contract(pair, provider).await
            }
        }
    }
}

impl Quoter for UniswapV2Quoter {
    fn id(&self) -> String {
        format!(
            "uniswap_v2:{}:{}:{}",
            self.pair_address, self.token0, self.token1
        )
    }

    fn tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        (
            TokenIdentifier::ERC20 {
                address: self.token0,
            },
            TokenIdentifier::ERC20 {
                address: self.token1,
            },
        )
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> Result<U256> {
        let pair = UniswapV2Pair::new(self.pair_address, &self.provider);
        let reserves = pair.getReserves().call().block(block.into()).await?;
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
