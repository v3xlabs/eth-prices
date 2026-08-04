//! Shared data model and scoring for pool discovery.
//!
//! Protocol-specific discovery lives with each quoter (for example
//! [`crate::quoter::uniswap_v2::discovery`]); this module holds what those
//! discoverers have in common: the discovered-pool representation, failure
//! reporting, deduplication, and the confidence model.

use std::collections::HashMap;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256},
    providers::Provider,
};

use crate::{asset::erc20::decimals_of, provider::RpcProvider, router::MAX_CONFIDENCE};

const FRESHNESS_HALF_LIFE_SECONDS: f64 = 86_400.0;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFailure {
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscovererReport {
    pub identity: String,
    pub attempted: usize,
    pub discovered: usize,
    pub skipped: usize,
    pub failures: Vec<DiscoveryFailure>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryReport {
    pub discoverers: Vec<DiscovererReport>,
}

/// The protocol-specific liquidity measurement backing a discovered pool.
#[derive(Debug, Clone)]
pub enum PoolLiquidity {
    /// Uniswap v2 style constant-product reserves.
    V2 {
        /// Raw `token0` reserve.
        reserve0: U256,
        /// Raw `token1` reserve.
        reserve1: U256,
    },
    /// Uniswap v3 style concentrated liquidity.
    V3 {
        /// The pool's in-range liquidity `L`.
        liquidity: u128,
    },
}

/// A pool found by a protocol discoverer, before deduplication and scoring.
#[derive(Debug, Clone)]
pub struct DiscoveredPool {
    /// The pool contract address.
    pub pool_address: Address,
    /// The pool's first token.
    pub token0: Address,
    /// The pool's second token.
    pub token1: Address,
    /// Raw ranking value for same-pair deduplication and the `min_liquidity`
    /// filter: the smaller V2 reserve, or the V3 in-range liquidity.
    pub score: U256,
    /// The protocol-specific liquidity measurement.
    pub liquidity: PoolLiquidity,
    /// Unix timestamp of the pool's most recent trade, when known.
    pub last_trade_timestamp: Option<u64>,
}

/// Splits per-target discovery results into found pools and failures,
/// dropping targets that had no pool.
pub fn collect_discoveries(
    results: Vec<Result<Option<DiscoveredPool>, DiscoveryFailure>>,
) -> (Vec<DiscoveredPool>, Vec<DiscoveryFailure>) {
    let mut pools = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(Some(pool)) => pools.push(pool),
            Ok(None) => {}
            Err(failure) => failures.push(failure),
        }
    }
    (pools, failures)
}

fn sorted_pair(a: Address, b: Address) -> (Address, Address) {
    if a < b { (a, b) } else { (b, a) }
}

/// Keeps the highest-scoring pool per token pair, in a deterministic order.
pub fn deduplicate_pools(pools: Vec<DiscoveredPool>) -> Vec<DiscoveredPool> {
    let mut best: HashMap<(Address, Address), DiscoveredPool> = HashMap::new();

    for pool in pools {
        let key = sorted_pair(pool.token0, pool.token1);
        match best.get(&key) {
            Some(existing) if existing.score >= pool.score => continue,
            _ => {
                best.insert(key, pool);
            }
        }
    }

    let mut pools: Vec<DiscoveredPool> = best.into_values().collect();
    pools.sort_by_key(|pool| pool.pool_address);
    pools
}

/// Drops pools whose `score` is below `min_liquidity`.
pub fn filter_pools(
    pools: Vec<DiscoveredPool>,
    min_liquidity: &Option<U256>,
) -> Vec<DiscoveredPool> {
    match min_liquidity {
        Some(min) => pools.into_iter().filter(|p| p.score >= *min).collect(),
        None => pools,
    }
}

fn approx_whole_units(value: U256, decimals: u8) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::INFINITY) / 10f64.powi(i32::from(decimals))
}

// The pool's liquidity as a geometric mean in whole token units, so pools are
// comparable across tokens with different decimals and across protocols: V3's
// L is sqrt(x·y) over the virtual reserves, the direct analogue of the V2
// sqrt of the reserve product.
fn geometric_mean_units(pool: &DiscoveredPool, decimals: &HashMap<Address, u8>) -> f64 {
    let decimals0 = decimals_of(decimals, pool.token0);
    let decimals1 = decimals_of(decimals, pool.token1);
    match pool.liquidity {
        PoolLiquidity::V2 { reserve0, reserve1 } => {
            let units0 = approx_whole_units(reserve0, decimals0);
            let units1 = approx_whole_units(reserve1, decimals1);
            (units0 * units1).sqrt()
        }
        PoolLiquidity::V3 { liquidity } => {
            let scale = 10f64.powf(f64::from(u32::from(decimals0) + u32::from(decimals1)) / 2.0);
            liquidity as f64 / scale
        }
    }
}

// Maps geometric-mean liquidity in whole token units onto 0..100, log-scaled
// so pools rank by order of magnitude; saturates at one million whole units.
fn liquidity_confidence(geometric_mean: f64) -> f64 {
    if !geometric_mean.is_finite() || geometric_mean <= 0.0 {
        return 0.0;
    }
    ((100.0 / 6.0) * (1.0 + geometric_mean).log10()).clamp(0.0, MAX_CONFIDENCE as f64)
}

// A pool that has not traded recently carries a spot price nobody has been
// willing to arbitrage, so its validity decays with the age of the last trade.
fn freshness_multiplier(age_seconds: f64) -> f64 {
    if !age_seconds.is_finite() || age_seconds <= 0.0 {
        return 1.0;
    }
    2f64.powf(-age_seconds / FRESHNESS_HALF_LIFE_SECONDS)
}

/// Scores a pool's spot-price trustworthiness on `0..=100`: decimals-normalized
/// geometric-mean liquidity on a log scale, decayed by the age of the pool's
/// last trade (24 hour half-life). Missing decimals fall back to 18 and a
/// missing timestamp counts as fresh — either degrades ranking, never quoting.
pub fn pool_confidence(
    pool: &DiscoveredPool,
    decimals: &HashMap<Address, u8>,
    block_timestamp: Option<u64>,
) -> u64 {
    let freshness = match (block_timestamp, pool.last_trade_timestamp) {
        (Some(now), Some(last_trade)) => {
            freshness_multiplier(now.saturating_sub(last_trade) as f64)
        }
        _ => 1.0,
    };
    (liquidity_confidence(geometric_mean_units(pool, decimals)) * freshness).round() as u64
}

/// Reads the timestamp of the given block, or of the chain head when `None`.
pub async fn fetch_block_timestamp(
    provider: &RpcProvider,
    block_number: Option<u64>,
) -> Option<u64> {
    provider
        .get_block_by_number(
            block_number.map_or(BlockNumberOrTag::Latest, BlockNumberOrTag::Number),
        )
        .await
        .ok()
        .flatten()
        .map(|block| block.header.timestamp)
}
