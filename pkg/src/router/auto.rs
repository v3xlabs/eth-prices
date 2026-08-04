use std::collections::HashSet;

use alloy::{
    eips::BlockId,
    primitives::{Address, U256, address},
};

use crate::{
    Result,
    asset::{erc20::fetch_decimals, identity::AssetIdentifier},
    network::NetworkId,
    provider::RpcProvider,
    quoter::{
        AnyQuoter, erc4626, uniswap_v2, uniswap_v2::UniswapV2Quoter, uniswap_v3,
        uniswap_v3::UniswapV3Quoter,
    },
    router::{
        Router,
        discovery::{
            DiscoveredPool, DiscovererReport, DiscoveryReport, deduplicate_pools,
            fetch_block_timestamp, filter_pools, pool_confidence,
        },
    },
};

const UNISWAP_V2_FACTORY: Address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
const UNISWAP_V3_FACTORY: Address = address!("0x1F98431c8aD98523631AE4a59f267346ea31F984");
const DEFAULT_V3_FEES: &[u32] = &[100, 500, 3000, 10000];

#[derive(Debug, Clone)]
pub struct AutoRouter {
    provider: RpcProvider,
    tokens: Vec<AssetIdentifier>,
    network_id: Option<NetworkId>,
    block_number: Option<u64>,
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
            block_number: None,
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

    /// Pin discovery to a specific block; defaults to the latest state.
    pub fn with_block_number(mut self, block_number: u64) -> Self {
        self.block_number = Some(block_number);
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
        let block = self.block_number.map_or(BlockId::latest(), BlockId::number);

        let mut all_quoters: Vec<AnyQuoter> = Vec::new();
        let mut report = DiscoveryReport::default();

        // 1. ERC4626 discovery first — collect underlying tokens
        let mut extra_addresses: Vec<Address> = Vec::new();
        if self.discover_erc4626 {
            let (erc4626_quoters, underlying, discoverer) = erc4626::discovery::discover_quoters(
                &self.provider,
                &network_id,
                self.erc20_addresses(),
                block,
            )
            .await;
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
            let (pools, attempted, failures) = uniswap_v2::discovery::discover_pools(
                &self.provider,
                &all_addresses,
                factory,
                block,
            )
            .await;
            v2_pools = filter_pools(deduplicate_pools(pools), &self.min_liquidity);
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
            let (pools, attempted, failures) = uniswap_v3::discovery::discover_pools(
                &self.provider,
                &all_addresses,
                factory,
                &self.uniswap_v3_fees,
                block,
            )
            .await;
            v3_pools = filter_pools(deduplicate_pools(pools), &self.min_liquidity);
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
            fetch_decimals(&self.provider, &all_addresses, block),
            fetch_block_timestamp(&self.provider, self.block_number),
            uniswap_v3::discovery::fetch_last_trades(&self.provider, &v3_pools, block),
        );

        for pool in v2_pools {
            let confidence = pool_confidence(&pool, &decimals, block_timestamp);
            let quoter = UniswapV2Quoter {
                network_id: network_id.clone(),
                pair_address: pool.pool_address,
                token0: pool.token0,
                token1: pool.token1,
            };
            all_quoters.push(AnyQuoter::from(quoter).with_confidence(confidence));
        }

        for pool in v3_pools {
            let pool = DiscoveredPool {
                last_trade_timestamp: v3_last_trades.get(&pool.pool_address).copied(),
                ..pool
            };
            let confidence = pool_confidence(&pool, &decimals, block_timestamp);
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
}
