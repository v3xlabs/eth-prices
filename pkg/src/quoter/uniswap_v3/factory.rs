use crate::Result;
use alloy::{
    primitives::{Address, address, aliases::U24},
    providers::DynProvider,
    sol,
};
use serde::Deserialize;

/// Configuration for a set of Uniswap v3 pools on a single chain.
#[derive(Debug, Deserialize, PartialEq)]
pub struct UniswapV3Config {
    /// Pools to load as quoters.
    pub pools: Vec<UniswapV3Selector>,
}

/// Selects a Uniswap v3 pool by address.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    serde(rename_all = "camelCase"),
    tsify(from_wasm_abi)
)]
pub enum UniswapV3Selector {
    /// Resolve the pool address from factory
    ByTokens {
        #[cfg_attr(target_arch = "wasm32", serde(rename = "tokenIn"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        token_in: Address,
        #[cfg_attr(target_arch = "wasm32", serde(rename = "tokenOut"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        token_out: Address,
        fee: Option<u32>,
    },
    /// Use an already-known pool contract address.
    Pool {
        #[cfg_attr(target_arch = "wasm32", serde(rename = "poolAddress"))]
        #[cfg_attr(target_arch = "wasm32", tsify(type = "string"))]
        pool_address: Address,
    },
}

sol! {
    #[sol(rpc)]
    contract UniswapV3Factory {
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool);
    }
}

impl UniswapV3Selector {
    pub async fn resolve(&self, provider: &DynProvider) -> Result<Address> {
        let factory_address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
        match self {
            UniswapV3Selector::ByTokens {
                token_in,
                token_out,
                fee,
            } => {
                let factory = UniswapV3Factory::new(factory_address, provider);
                let fee = U24::from(fee.unwrap_or(3000));
                let pool = factory.getPool(*token_in, *token_out, fee).call().await?;
                Ok(pool)
            }
            UniswapV3Selector::Pool { pool_address } => Ok(*pool_address),
        }
    }
}

// pub async fn fetch_pools(provider: &DynProvider, factory_address: Address, fees: Vec<U24>) -> Vec<Address> {
//     // [500, 3000, 10000]
// }

#[cfg(test)]
mod tests {
    // const FACTORY_ADDRESS: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
}
