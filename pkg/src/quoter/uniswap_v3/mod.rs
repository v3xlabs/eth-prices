//! Uniswap v3 quote sources.

use alloy::{
    primitives::{Address, BlockNumber, U256, U512},
    providers::DynProvider,
};
use anyhow::Result;
use pool::UniswapV3Pool;

use crate::{
    quoter::{Quoter, RateDirection, uniswap_v3::factory::UniswapV3Selector},
    token::identity::TokenIdentifier,
};

pub mod factory;
pub mod pool;


/// Quotes spot rates from a Uniswap v3 pool at a given block height.
#[derive(Debug, Clone)]
pub struct UniswapV3Quoter {
    /// Pool contract address.
    pub pool_address: Address,
    /// First token in pool order.
    pub token0: Address,
    /// Second token in pool order.
    pub token1: Address,
    /// Provider used to fetch historical pool state.
    pub provider: DynProvider,
}

impl UniswapV3Quoter {
    /// Builds a quoter from a configured pool selector.
    pub async fn from_selector(provider: &DynProvider, selector: UniswapV3Selector) -> Self {
        let pool_address = selector.resolve(provider).await.unwrap();
        let pool = UniswapV3Pool::new(pool_address, provider);
        let token0 = pool.token0().call().await.unwrap();
        let token1 = pool.token1().call().await.unwrap();
        Self {
            pool_address,
            token0,
            token1,
            provider: provider.clone(),
        }
    }
}

impl Quoter for UniswapV3Quoter {
    fn get_slug(&self) -> String {
        format!("uniswap_v3:{}", self.pool_address)
    }

    fn get_tokens(&self) -> (TokenIdentifier, TokenIdentifier) {
        (self.token0.into(), self.token1.into())
    }

    async fn get_rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        block: BlockNumber,
    ) -> Result<U256> {
        let pool = UniswapV3Pool::new(self.pool_address, &self.provider);
        let slot0 = pool.slot0().block(block.into()).call().await?;
        let sqrt_price_x96 = U512::from(slot0.sqrtPriceX96);
        let q192 = U512::from(1) << 192;
        let sqrt_price_squared = sqrt_price_x96 * sqrt_price_x96;

        let price0_in_1_raw = (sqrt_price_squared * U512::from(amount_in)) / q192;
        let price1_in_0_raw = (q192 * U512::from(amount_in)) / sqrt_price_squared;

        Ok(match direction {
            RateDirection::Forward => U256::from(price0_in_1_raw),
            RateDirection::Reverse => U256::from(price1_in_0_raw),
        })
    }
}
