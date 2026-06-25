use std::collections::HashSet;

use alloy::{
    primitives::{Address, U256, address, aliases::U24},
};
use futures::future::join_all;

use crate::{
    Result,
    asset::identity::AssetIdentifier,
    network::NetworkId,
    provider::RpcProvider,
    quoter::{
        AnyQuoter,
        erc4626::{ERC4626, ERC4626Quoter},
        uniswap_v2::{
            UniswapV2Quoter,
            factory::UniswapV2Factory,
            pair::UniswapV2Pair,
        },
        uniswap_v3::{
            UniswapV3Quoter,
            factory::UniswapV3Factory,
            pool::UniswapV3Pool,
        },
    },
    router::Router,
};

const UNISWAP_V2_FACTORY: Address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
const UNISWAP_V3_FACTORY: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
const DEFAULT_V3_FEES: &[u32] = &[100, 500, 3000, 10000];
const MAX_CONFIDENCE: u64 = 100;

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
        let network_id = match self.network_id {
            Some(ref id) => id.clone(),
            None => NetworkId::from_provider(&self.provider).await?,
        };

        let mut all_quoters: Vec<AnyQuoter> = Vec::new();

        // 1. ERC4626 discovery first — collect underlying tokens
        let mut extra_addresses: Vec<Address> = Vec::new();
        if self.discover_erc4626 {
            let (erc4626_quoters, underlying) = self.discover_erc4626_quoters(&network_id).await;
            all_quoters.extend(erc4626_quoters);
            extra_addresses = underlying;
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
            v2_pools = Self::discover_v2_pools_inner(
                &self.provider,
                &all_addresses,
                self.uniswap_v2_factory,
            )
            .await;
            v2_pools = Self::filter_pools(Self::deduplicate_pools(v2_pools), &self.min_liquidity);
        }

        // 4. V3 discovery with expanded set
        let mut v3_pools: Vec<DiscoveredPool> = Vec::new();
        if self.discover_v3 {
            v3_pools = Self::discover_v3_pools_inner(
                &self.provider,
                &all_addresses,
                self.uniswap_v3_factory,
                &self.uniswap_v3_fees,
            )
            .await;
            v3_pools = Self::filter_pools(Self::deduplicate_pools(v3_pools), &self.min_liquidity);
        }

        // 5. Build quoters — V2 first so the Router's .find() prefers them
        //    for direct routes (V2 spot prices are generally more reliable for
        //    thin pools). V3 quoters remain as fallback for multi-hop paths.
        for pool in v2_pools {
            let quoter = UniswapV2Quoter {
                network_id: network_id.clone(),
                pair_address: pool.pool_address,
                token0: pool.token0,
                token1: pool.token1,
            };
            let confidence = pool_confidence_v2(pool.score);
            all_quoters.push(AnyQuoter::from(quoter).with_confidence(confidence));
        }

        for pool in v3_pools {
            let quoter = UniswapV3Quoter {
                network_id: network_id.clone(),
                pool_address: pool.pool_address,
                token0: pool.token0,
                token1: pool.token1,
            };
            let confidence = pool_confidence_v3(pool.score);
            all_quoters.push(AnyQuoter::from(quoter).with_confidence(confidence));
        }

        if all_quoters.is_empty() {
            return Err(crate::error::EthPricesError::AutoRouterNoPools);
        }

        Ok(Router::from_iter(all_quoters))
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

        best.into_values().collect()
    }

    fn filter_pools(pools: Vec<DiscoveredPool>, min_liquidity: &Option<U256>) -> Vec<DiscoveredPool> {
        match min_liquidity {
            Some(min) => pools.into_iter().filter(|p| p.score >= *min).collect(),
            None => pools,
        }
    }

    async fn discover_v2_pools_inner(
        provider: &RpcProvider,
        addresses: &[Address],
        factory_opt: Option<Address>,
    ) -> Vec<DiscoveredPool> {
        let factory = factory_opt.unwrap_or(UNISWAP_V2_FACTORY);

        let mut pairs = Vec::new();
        for i in 0..addresses.len() {
            for j in (i + 1)..addresses.len() {
                pairs.push((addresses[i], addresses[j]));
            }
        }

        let results: Vec<_> = join_all(pairs.into_iter().map(|(a, b)| {
            let provider = provider.clone();
            async move {
                discover_single_v2_pool(&provider, factory, a, b).await
            }
        }))
        .await;

        let pools: Vec<DiscoveredPool> = results.into_iter().flatten().collect();

        let liq_futures: Vec<_> = pools.iter().map(|pool| {
            let provider = provider.clone();
            async move {
                let pair = UniswapV2Pair::new(pool.pool_address, &provider);
                match pair.getReserves().call().await {
                    Ok(reserves) => {
                        let reserve0 = U256::from(reserves.reserve0);
                        let reserve1 = U256::from(reserves.reserve1);
                        Some(std::cmp::min(reserve0, reserve1))
                    }
                    Err(_) => None,
                }
            }
        }).collect();

        let scores: Vec<Option<U256>> = join_all(liq_futures).await;

        pools
            .into_iter()
            .zip(scores.into_iter())
            .filter_map(|(mut pool, score)| {
                pool.score = score?;
                Some(pool)
            })
            .collect()
    }

    async fn discover_v3_pools_inner(
        provider: &RpcProvider,
        addresses: &[Address],
        factory_opt: Option<Address>,
        fees: &[u32],
    ) -> Vec<DiscoveredPool> {
        let factory = factory_opt.unwrap_or(UNISWAP_V3_FACTORY);

        if addresses.len() < 2 {
            return Vec::new();
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

        join_all(queries.into_iter().map(|(a, b, fee)| {
            let provider = provider.clone();
            async move {
                discover_single_v3_pool(&provider, factory, a, b, fee).await
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    }

    async fn discover_erc4626_quoters(&self, network_id: &NetworkId) -> (Vec<AnyQuoter>, Vec<Address>) {
        let addresses = self.erc20_addresses();

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
                        Some((AnyQuoter::from(quoter).with_confidence(50), underlying))
                    }
                    Err(_) => None,
                }
            }
        }))
        .await;

        let (quoters, underlying): (Vec<_>, Vec<_>) = results
            .into_iter()
            .flatten()
            .unzip();

        (quoters, underlying)
    }
}

fn pool_confidence_v2(score: U256) -> u64 {
    if score.is_zero() {
        return 0;
    }
    let divisor = U256::from(1_000_000_000u64);
    let scaled = score / divisor;
    if scaled >= U256::from(MAX_CONFIDENCE) {
        MAX_CONFIDENCE
    } else {
        scaled.as_limbs()[0] as u64
    }
}

fn pool_confidence_v3(score: U256) -> u64 {
    if score.is_zero() {
        return 0;
    }
    let divisor = U256::from(10_000_000_000_000_000u64);
    let scaled = score / divisor;
    if scaled >= U256::from(MAX_CONFIDENCE) {
        MAX_CONFIDENCE
    } else {
        scaled.as_limbs()[0] as u64
    }
}

async fn discover_single_v2_pool(
    provider: &RpcProvider,
    factory: Address,
    token_a: Address,
    token_b: Address,
) -> Option<DiscoveredPool> {
    let v2_factory = UniswapV2Factory::new(factory, provider);
    let pair = v2_factory.getPair(token_a, token_b).call().await.ok()?;
    if pair.is_zero() {
        return None;
    }

    let pair_contract = UniswapV2Pair::new(pair, provider);
    let token0 = pair_contract.token0().call().await.ok()?;
    let token1 = pair_contract.token1().call().await.ok()?;

    Some(DiscoveredPool {
        pool_address: pair,
        token0,
        token1,
        score: U256::ZERO,
        kind: PoolKind::V2,
    })
}

async fn discover_single_v3_pool(
    provider: &RpcProvider,
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee: u32,
) -> Option<DiscoveredPool> {
    let v3_factory = UniswapV3Factory::new(factory, provider);
    let pool = v3_factory
        .getPool(token_a, token_b, U24::from(fee))
        .call()
        .await
        .ok()?;
    if pool.is_zero() {
        return None;
    }

    let pool_contract = UniswapV3Pool::new(pool, provider);
    let token0 = pool_contract.token0().call().await.ok()?;
    let token1 = pool_contract.token1().call().await.ok()?;
    let liq: u128 = pool_contract.liquidity().call().await.ok()?;

    Some(DiscoveredPool {
        pool_address: pool,
        token0,
        token1,
        score: U256::from(liq),
        kind: PoolKind::V3(fee),
    })
}