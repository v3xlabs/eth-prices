use std::collections::{HashMap, HashSet};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256, address, aliases::U24},
    providers::Provider,
};
use futures::future::join_all;

use crate::{
    Result,
    asset::{erc20::ERC20, identity::AssetIdentifier},
    network::NetworkId,
    provider::RpcProvider,
    quoter::{
        AnyQuoter,
        erc4626::{ERC4626, ERC4626Quoter},
        uniswap_v2::{UniswapV2Quoter, discovery::UniswapV2Factory, pair::UniswapV2Pair},
        uniswap_v3::{UniswapV3Quoter, discovery::UniswapV3Factory, pool::UniswapV3Pool},
    },
    router::Router,
};

const UNISWAP_V2_FACTORY: Address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
const UNISWAP_V3_FACTORY: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
const DEFAULT_V3_FEES: &[u32] = &[100, 500, 3000, 10000];
const MAX_CONFIDENCE: u64 = 100;
const FALLBACK_DECIMALS: u8 = 18;
const FRESHNESS_HALF_LIFE_SECONDS: f64 = 86_400.0;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PoolKind {
    V2,
    V3(u32),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DiscoveredPool {
    pool_address: Address,
    token0: Address,
    token1: Address,
    score: U256,
    kind: PoolKind,
    reserve0: U256,
    reserve1: U256,
    last_trade_timestamp: Option<u64>,
}

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

#[derive(Debug, Clone)]
pub struct AutoRouter {
    provider: RpcProvider,
    tokens: Vec<AssetIdentifier>,
    network_id: Option<NetworkId>,
    uniswap_v2_factory: Option<Address>,
    uniswap_v3_factory: Option<Address>,
    uniswap_v3_fees: Vec<u32>,
    min_liquidity: Option<U256>,
    discover_v2: bool,
    discover_v3: bool,
    discover_erc4626: bool,
}

impl AutoRouter {
    pub fn new(provider: RpcProvider, tokens: Vec<AssetIdentifier>) -> Self {
        Self {
            provider,
            tokens,
            network_id: None,
            uniswap_v2_factory: None,
            uniswap_v3_factory: None,
            uniswap_v3_fees: DEFAULT_V3_FEES.to_vec(),
            min_liquidity: Some(U256::from(1)),
            discover_v2: true,
            discover_v3: true,
            discover_erc4626: true,
        }
    }

    pub fn with_network_id(mut self, network_id: NetworkId) -> Self {
        self.network_id = Some(network_id);
        self
    }

    pub fn with_uniswap_v2_factory(mut self, address: Address) -> Self {
        self.uniswap_v2_factory = Some(address);
        self
    }

    pub fn with_uniswap_v3_factory(mut self, address: Address) -> Self {
        self.uniswap_v3_factory = Some(address);
        self
    }

    pub fn with_uniswap_v3_fees(mut self, fees: Vec<u32>) -> Self {
        self.uniswap_v3_fees = fees;
        self
    }

    pub fn with_min_liquidity(mut self, min: U256) -> Self {
        self.min_liquidity = Some(min);
        self
    }

    pub fn discover_uniswap_v2(mut self, enable: bool) -> Self {
        self.discover_v2 = enable;
        self
    }

    pub fn discover_uniswap_v3(mut self, enable: bool) -> Self {
        self.discover_v3 = enable;
        self
    }

    pub fn discover_erc4626(mut self, enable: bool) -> Self {
        self.discover_erc4626 = enable;
        self
    }

    pub async fn build(self) -> Result<Router> {
        Ok(self.build_with_report().await?.0)
    }

    pub async fn build_with_report(self) -> Result<(Router, DiscoveryReport)> {
        let network_id = match self.network_id {
            Some(ref id) => id.clone(),
            None => NetworkId::from_provider(&self.provider).await?,
        };

        let mut all_quoters: Vec<AnyQuoter> = Vec::new();
        let mut report = DiscoveryReport::default();

        // 1. ERC4626 discovery first — collect underlying tokens
        let mut extra_addresses: Vec<Address> = Vec::new();
        if self.discover_erc4626 {
            let (erc4626_quoters, underlying, discoverer) =
                self.discover_erc4626_quoters(&network_id).await;
            all_quoters.extend(erc4626_quoters);
            extra_addresses = underlying;
            report.discoverers.push(discoverer);
        }

        // 2. Build expanded address set (input tokens + ERC4626 underlyings)
        let mut all_addresses: Vec<Address> = self.erc20_addresses();
        let existing: HashSet<Address> = all_addresses.iter().copied().collect();
        for addr in extra_addresses {
            if !existing.contains(&addr) {
                all_addresses.push(addr);
            }
        }

        // 3. V2 discovery with expanded set
        let mut v2_pools: Vec<DiscoveredPool> = Vec::new();
        if self.discover_v2 {
            let factory = self.uniswap_v2_factory.unwrap_or(UNISWAP_V2_FACTORY);
            let (pools, attempted, failures) =
                Self::discover_v2_pools_inner(&self.provider, &all_addresses, factory).await;
            v2_pools = Self::filter_pools(Self::deduplicate_pools(pools), &self.min_liquidity);
            report.discoverers.push(DiscovererReport {
                identity: format!("uniswap_v2:{factory}"),
                attempted,
                discovered: v2_pools.len(),
                skipped: attempted.saturating_sub(v2_pools.len() + failures.len()),
                failures,
            });
        }

        // 4. V3 discovery with expanded set
        let mut v3_pools: Vec<DiscoveredPool> = Vec::new();
        if self.discover_v3 {
            let factory = self.uniswap_v3_factory.unwrap_or(UNISWAP_V3_FACTORY);
            let (pools, attempted, failures) = Self::discover_v3_pools_inner(
                &self.provider,
                &all_addresses,
                factory,
                &self.uniswap_v3_fees,
            )
            .await;
            v3_pools = Self::filter_pools(Self::deduplicate_pools(pools), &self.min_liquidity);
            report.discoverers.push(DiscovererReport {
                identity: format!("uniswap_v3:{factory}"),
                attempted,
                discovered: v3_pools.len(),
                skipped: attempted.saturating_sub(v3_pools.len() + failures.len()),
                failures,
            });
        }

        // 5. Score every kept pool: decimals-normalized liquidity decayed by
        //    the age of the pool's last trade, matching the TypeScript router.
        let (decimals, block_timestamp, v3_last_trades) = futures::join!(
            fetch_decimals(&self.provider, &all_addresses),
            fetch_block_timestamp(&self.provider),
            fetch_v3_last_trades(&self.provider, &v3_pools),
        );

        for pool in v2_pools {
            let confidence = pool_confidence(
                v2_geometric_mean(&pool, &decimals),
                block_timestamp,
                pool.last_trade_timestamp,
            );
            let quoter = UniswapV2Quoter {
                network_id: network_id.clone(),
                pair_address: pool.pool_address,
                token0: pool.token0,
                token1: pool.token1,
            };
            all_quoters.push(AnyQuoter::from(quoter).with_confidence(confidence));
        }

        for pool in v3_pools {
            let confidence = pool_confidence(
                v3_geometric_mean(&pool, &decimals),
                block_timestamp,
                v3_last_trades.get(&pool.pool_address).copied(),
            );
            let quoter = UniswapV3Quoter {
                network_id: network_id.clone(),
                pool_address: pool.pool_address,
                token0: pool.token0,
                token1: pool.token1,
            };
            all_quoters.push(AnyQuoter::from(quoter).with_confidence(confidence));
        }

        if all_quoters.is_empty() {
            return Err(crate::error::EthPricesError::AutoRouterNoPools);
        }

        Ok((Router::from_iter(all_quoters), report))
    }

    fn erc20_addresses(&self) -> Vec<Address> {
        self.tokens
            .iter()
            .filter_map(|t| match t {
                AssetIdentifier::ERC20 { address } => Some(*address),
                _ => None,
            })
            .collect()
    }

    fn sorted_pair(a: Address, b: Address) -> (Address, Address) {
        if a < b { (a, b) } else { (b, a) }
    }

    fn deduplicate_pools(pools: Vec<DiscoveredPool>) -> Vec<DiscoveredPool> {
        let mut best: std::collections::HashMap<(Address, Address), DiscoveredPool> =
            std::collections::HashMap::new();

        for pool in pools {
            let key = Self::sorted_pair(pool.token0, pool.token1);
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

    fn filter_pools(
        pools: Vec<DiscoveredPool>,
        min_liquidity: &Option<U256>,
    ) -> Vec<DiscoveredPool> {
        match min_liquidity {
            Some(min) => pools.into_iter().filter(|p| p.score >= *min).collect(),
            None => pools,
        }
    }

    async fn discover_v2_pools_inner(
        provider: &RpcProvider,
        addresses: &[Address],
        factory: Address,
    ) -> (Vec<DiscoveredPool>, usize, Vec<DiscoveryFailure>) {
        let mut pairs = Vec::new();
        for i in 0..addresses.len() {
            for j in (i + 1)..addresses.len() {
                pairs.push((addresses[i], addresses[j]));
            }
        }
        let attempted = pairs.len();

        let results: Vec<_> = join_all(pairs.into_iter().map(|(a, b)| {
            let provider = provider.clone();
            async move { discover_single_v2_pool(&provider, factory, a, b).await }
        }))
        .await;

        let mut pools: Vec<DiscoveredPool> = Vec::new();
        let mut failures: Vec<DiscoveryFailure> = Vec::new();
        for result in results {
            match result {
                Ok(Some(pool)) => pools.push(pool),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }

        let liq_futures: Vec<_> = pools
            .iter()
            .map(|pool| {
                let provider = provider.clone();
                async move {
                    let pair = UniswapV2Pair::new(pool.pool_address, &provider);
                    pair.getReserves()
                        .call()
                        .await
                        .map(|reserves| {
                            (
                                U256::from(reserves.reserve0),
                                U256::from(reserves.reserve1),
                                u64::from(reserves.blockTimestampLast),
                            )
                        })
                        .map_err(|error| error.to_string())
                }
            })
            .collect();

        let scores = join_all(liq_futures).await;
        let scored = pools
            .into_iter()
            .zip(scores)
            .filter_map(|(mut pool, reserves)| match reserves {
                Ok((reserve0, reserve1, last_trade)) => {
                    pool.score = std::cmp::min(reserve0, reserve1);
                    pool.reserve0 = reserve0;
                    pool.reserve1 = reserve1;
                    pool.last_trade_timestamp = Some(last_trade);
                    Some(pool)
                }
                Err(message) => {
                    failures.push(DiscoveryFailure {
                        target: pool.pool_address.to_string(),
                        message: format!("getReserves failed: {message}"),
                    });
                    None
                }
            })
            .collect();

        (scored, attempted, failures)
    }

    async fn discover_v3_pools_inner(
        provider: &RpcProvider,
        addresses: &[Address],
        factory: Address,
        fees: &[u32],
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
            async move { discover_single_v3_pool(&provider, factory, a, b, fee).await }
        }))
        .await;

        let mut pools: Vec<DiscoveredPool> = Vec::new();
        let mut failures: Vec<DiscoveryFailure> = Vec::new();
        for result in results {
            match result {
                Ok(Some(pool)) => pools.push(pool),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }

        (pools, attempted, failures)
    }

    async fn discover_erc4626_quoters(
        &self,
        network_id: &NetworkId,
    ) -> (Vec<AnyQuoter>, Vec<Address>, DiscovererReport) {
        let addresses = self.erc20_addresses();
        let attempted = addresses.len();

        let results: Vec<_> = join_all(addresses.into_iter().map(|addr| {
            let provider = self.provider.clone();
            let net_id = network_id.clone();
            async move {
                match ERC4626::new(addr, &provider).asset().call().await {
                    Ok(underlying) => {
                        let quoter = ERC4626Quoter {
                            network_id: net_id,
                            vault_address: AssetIdentifier::ERC20 { address: addr },
                            token_address: AssetIdentifier::ERC20 {
                                address: underlying,
                            },
                        };
                        Ok((AnyQuoter::from(quoter).with_confidence(50), underlying))
                    }
                    Err(error) => Err(DiscoveryFailure {
                        target: addr.to_string(),
                        message: error.to_string(),
                    }),
                }
            }
        }))
        .await;

        let mut quoters = Vec::new();
        let mut underlying = Vec::new();
        let mut failures = Vec::new();
        for result in results {
            match result {
                Ok((quoter, asset)) => {
                    quoters.push(quoter);
                    underlying.push(asset);
                }
                Err(failure) => failures.push(failure),
            }
        }

        let discovered = quoters.len();
        (
            quoters,
            underlying,
            DiscovererReport {
                identity: format!("erc4626:{}", network_id.0),
                attempted,
                discovered,
                skipped: failures.len(),
                failures,
            },
        )
    }
}

fn approx_whole_units(value: U256, decimals: u8) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(f64::INFINITY) / 10f64.powi(i32::from(decimals))
}

fn decimals_of(decimals: &HashMap<Address, u8>, token: Address) -> u8 {
    decimals.get(&token).copied().unwrap_or(FALLBACK_DECIMALS)
}

fn v2_geometric_mean(pool: &DiscoveredPool, decimals: &HashMap<Address, u8>) -> f64 {
    let units0 = approx_whole_units(pool.reserve0, decimals_of(decimals, pool.token0));
    let units1 = approx_whole_units(pool.reserve1, decimals_of(decimals, pool.token1));
    (units0 * units1).sqrt()
}

// V3 liquidity L is sqrt(x·y) over the virtual reserves, so normalizing by the
// average token decimals yields the same whole-unit quantity scored for V2.
fn v3_geometric_mean(pool: &DiscoveredPool, decimals: &HashMap<Address, u8>) -> f64 {
    let scale = 10f64.powf(
        f64::from(
            u32::from(decimals_of(decimals, pool.token0))
                + u32::from(decimals_of(decimals, pool.token1)),
        ) / 2.0,
    );
    approx_whole_units(pool.score, 0) / scale
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

fn pool_confidence(
    geometric_mean: f64,
    block_timestamp: Option<u64>,
    last_trade_timestamp: Option<u64>,
) -> u64 {
    let freshness = match (block_timestamp, last_trade_timestamp) {
        (Some(now), Some(last_trade)) => {
            freshness_multiplier(now.saturating_sub(last_trade) as f64)
        }
        _ => 1.0,
    };
    (liquidity_confidence(geometric_mean) * freshness).round() as u64
}

async fn fetch_decimals(provider: &RpcProvider, addresses: &[Address]) -> HashMap<Address, u8> {
    join_all(addresses.iter().map(|address| {
        let provider = provider.clone();
        let address = *address;
        async move {
            let decimals = ERC20::new(address, &provider).decimals().call().await;
            (address, decimals.unwrap_or(FALLBACK_DECIMALS))
        }
    }))
    .await
    .into_iter()
    .collect()
}

async fn fetch_block_timestamp(provider: &RpcProvider) -> Option<u64> {
    provider
        .get_block_by_number(BlockNumberOrTag::Latest)
        .await
        .ok()
        .flatten()
        .map(|block| block.header.timestamp)
}

// The most recent oracle observation is written on the first swap of a block,
// so its timestamp tells when the pool's spot price was last market-tested.
async fn fetch_v3_last_trades(
    provider: &RpcProvider,
    pools: &[DiscoveredPool],
) -> HashMap<Address, u64> {
    join_all(pools.iter().map(|pool| {
        let provider = provider.clone();
        let pool_address = pool.pool_address;
        async move {
            let contract = UniswapV3Pool::new(pool_address, &provider);
            let slot = contract.slot0().call().await.ok()?;
            let observation = contract
                .observations(U256::from(slot.observationIndex))
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

async fn discover_single_v2_pool(
    provider: &RpcProvider,
    factory: Address,
    token_a: Address,
    token_b: Address,
) -> std::result::Result<Option<DiscoveredPool>, DiscoveryFailure> {
    let failure = |target: String, message: String| DiscoveryFailure { target, message };
    let v2_factory = UniswapV2Factory::new(factory, provider);
    let pair = v2_factory
        .getPair(token_a, token_b)
        .call()
        .await
        .map_err(|error| {
            failure(
                format!("{token_a}/{token_b}"),
                format!("getPair failed: {error}"),
            )
        })?;
    if pair.is_zero() {
        return Ok(None);
    }

    let pair_contract = UniswapV2Pair::new(pair, provider);
    let token0 = pair_contract
        .token0()
        .call()
        .await
        .map_err(|error| failure(pair.to_string(), format!("token0 failed: {error}")))?;
    let token1 = pair_contract
        .token1()
        .call()
        .await
        .map_err(|error| failure(pair.to_string(), format!("token1 failed: {error}")))?;

    Ok(Some(DiscoveredPool {
        pool_address: pair,
        token0,
        token1,
        score: U256::ZERO,
        kind: PoolKind::V2,
        reserve0: U256::ZERO,
        reserve1: U256::ZERO,
        last_trade_timestamp: None,
    }))
}

async fn discover_single_v3_pool(
    provider: &RpcProvider,
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee: u32,
) -> std::result::Result<Option<DiscoveredPool>, DiscoveryFailure> {
    let failure = |target: String, message: String| DiscoveryFailure { target, message };
    let v3_factory = UniswapV3Factory::new(factory, provider);
    let pool = v3_factory
        .getPool(token_a, token_b, U24::from(fee))
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
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("token0 failed: {error}")))?;
    let token1 = pool_contract
        .token1()
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("token1 failed: {error}")))?;
    let liq: u128 = pool_contract
        .liquidity()
        .call()
        .await
        .map_err(|error| failure(pool.to_string(), format!("liquidity failed: {error}")))?;

    Ok(Some(DiscoveredPool {
        pool_address: pool,
        token0,
        token1,
        score: U256::from(liq),
        kind: PoolKind::V3(fee),
        reserve0: U256::ZERO,
        reserve1: U256::ZERO,
        last_trade_timestamp: None,
    }))
}
