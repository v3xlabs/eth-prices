use alloy::{primitives::{Address, address, aliases::U24}, providers::DynProvider, sol};
use anyhow::Result;
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
pub enum UniswapV3Selector {
    /// Resolve the pool address from factory
    ByTokens {
        token_in: Address,
        token_out: Address,
        fee: Option<u32>,
    },
    /// Use an already-known pool contract address.
    Pool { pool_address: Address },
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
            UniswapV3Selector::ByTokens { token_in, token_out, fee } => {
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
    use alloy::primitives::{Address, address};

    const FACTORY_ADDRESS: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
}
