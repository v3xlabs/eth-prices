use std::collections::HashMap;

use alloy::{
    eips::BlockId,
    primitives::{Address, U256, address, aliases::U24},
    sol,
};
use futures::future::join_all;
use serde::Deserialize;

use crate::{
    Result,
    provider::RpcProvider,
    quoter::uniswap_v3::pool::UniswapV3Pool,
    router::discovery::{DiscoveredPool, DiscoveryFailure, PoolLiquidity, collect_discoveries},
};

/// Configuration for a set of Uniswap v3 pools on a single chain.
#[derive(Debug, Deserialize, PartialEq, Clone)]
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
    pub async fn resolve(&self, provider: &RpcProvider) -> Result<Address> {
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

/// Probes the factory for every pairing of `addresses` at every fee tier and
/// returns the live pools with their in-range liquidity, the number of
/// pairings attempted, and any RPC failures.
pub async fn discover_pools(
    provider: &RpcProvider,
    addresses: &[Address],
    factory: Address,
    fees: &[u32],
    block: BlockId,
) -> (Vec<DiscoveredPool>, usize, Vec<DiscoveryFailure>) {
    if addresses.len() < 2 {
        return (Vec::new(), 0, Vec::new());
    }

    let mut queries = Vec::new();
    for i in 0..addresses.len() {
        for j in (i + 1)..addresses.len() {
            let a = addresses[i];
            let b = addresses[j];
            for &fee in fees {
                queries.push((a, b, fee));
            }
        }
    }
    let attempted = queries.len();

    let results = join_all(queries.into_iter().map(|(a, b, fee)| {
        let provider = provider.clone();
        async move { discover_single_pool(&provider, factory, a, b, fee, block).await }
    }))
    .await;

    let (pools, failures) = collect_discoveries(results);
    (pools, attempted, failures)
}

async fn discover_single_pool(
    provider: &RpcProvider,
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee: u32,
    block: BlockId,
) -> std::result::Result<Option<DiscoveredPool>, DiscoveryFailure> {
    let failure = |target: String, message: String| DiscoveryFailure { target, message };
    let v3_factory = UniswapV3Factory::new(factory, provider);
    let pool = v3_factory
        .getPool(token_a, token_b, U24::from(fee))
        .block(block)
        .call()
        .await
        .map_err(|error| {
            failure(
                format!("{token_a}/{token_b}@{fee}"),
                format!("getPool failed: {error}"),
            )
        })?;
    if pool.is_zero() {
        return Ok(None);
    }

    let pool_contract = UniswapV3Pool::new(pool, provider);
    let token0 = pool_contract
        .token0()
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("token0 failed: {error}")))?;
    let token1 = pool_contract
        .token1()
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("token1 failed: {error}")))?;
    let liquidity: u128 = pool_contract
        .liquidity()
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("liquidity failed: {error}")))?;

    Ok(Some(DiscoveredPool {
        pool_address: pool,
        token0,
        token1,
        score: U256::from(liquidity),
        liquidity: PoolLiquidity::V3 { liquidity },
        last_trade_timestamp: None,
    }))
}

/// Reads each pool's most recent oracle observation timestamp. Observations
/// are written on the first swap of a block, so this tells when the pool's
/// spot price was last market-tested.
pub async fn fetch_last_trades(
    provider: &RpcProvider,
    pools: &[DiscoveredPool],
    block: BlockId,
) -> HashMap<Address, u64> {
    join_all(pools.iter().map(|pool| {
        let provider = provider.clone();
        let pool_address = pool.pool_address;
        async move {
            let contract = UniswapV3Pool::new(pool_address, &provider);
            let slot = contract.slot0().block(block).call().await.ok()?;
            let observation = contract
                .observations(U256::from(slot.observationIndex))
                .block(block)
                .call()
                .await
                .ok()?;
            Some((pool_address, u64::from(observation.blockTimestamp)))
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    // const FACTORY_ADDRESS: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
}
