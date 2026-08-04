use alloy::{
    eips::BlockId,
    primitives::{Address, U256},
    sol,
};
use async_stream::stream;
use futures::{Stream, future::join_all};

use crate::{
    provider::RpcProvider,
    quoter::uniswap_v2::pair::UniswapV2Pair,
    router::discovery::{DiscoveredPool, DiscoveryFailure, PoolLiquidity, collect_discoveries},
};

sol! {
   #[sol(rpc)]
   contract UniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
        function allPairs(uint) external view returns (address pair);
        function allPairsLength() external view returns (uint);
   }
}

pub fn fetch_all_pairs<'a>(
    provider: &'a RpcProvider,
    factory_address: Address,
) -> impl Stream<Item = crate::Result<Address>> + 'a {
    stream! {
        let factory = UniswapV2Factory::new(factory_address, provider);
        let fr = &factory;
        let max = match fr.allPairsLength().call().await { Ok(m) => m, Err(e) => { yield Err(crate::error::EthPricesError::from(e)); return; } };

        let mut state = U256::from(0);
        while state < max {

            let pair = match fr.allPairs(U256::from(state)).call().await { Ok(p) => p, Err(e) => { yield Err(crate::error::EthPricesError::from(e)); return; } };

            yield Ok(pair);

            state += U256::from(1);
        }
    }
}

pub async fn fetch_pair(
    provider: &RpcProvider,
    factory_address: Address,
    token_from: Address,
    token_to: Address,
) -> crate::Result<Address> {
    let factory = UniswapV2Factory::new(factory_address, provider);
    let fr = &factory;

    let pair = fr.getPair(token_from, token_to).call().await?;
    Ok(pair)
}

/// Probes the factory for every pairing of `addresses` at `block` and returns
/// the live pools with their reserves, the number of pairings attempted, and
/// any RPC failures.
pub async fn discover_pools(
    provider: &RpcProvider,
    addresses: &[Address],
    factory: Address,
    block: BlockId,
) -> (Vec<DiscoveredPool>, usize, Vec<DiscoveryFailure>) {
    let mut pairs = Vec::new();
    for i in 0..addresses.len() {
        for j in (i + 1)..addresses.len() {
            pairs.push((addresses[i], addresses[j]));
        }
    }
    let attempted = pairs.len();

    let results = join_all(pairs.into_iter().map(|(a, b)| {
        let provider = provider.clone();
        async move { discover_single_pool(&provider, factory, a, b, block).await }
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
    block: BlockId,
) -> Result<Option<DiscoveredPool>, DiscoveryFailure> {
    let failure = |target: String, message: String| DiscoveryFailure { target, message };
    let v2_factory = UniswapV2Factory::new(factory, provider);
    let pair = v2_factory
        .getPair(token_a, token_b)
        .block(block)
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
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pair.to_string(), format!("token0 failed: {error}")))?;
    let token1 = pair_contract
        .token1()
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pair.to_string(), format!("token1 failed: {error}")))?;
    let reserves = pair_contract
        .getReserves()
        .block(block)
        .call()
        .await
        .map_err(|error| failure(pair.to_string(), format!("getReserves failed: {error}")))?;
    let reserve0 = U256::from(reserves.reserve0);
    let reserve1 = U256::from(reserves.reserve1);

    Ok(Some(DiscoveredPool {
        pool_address: pair,
        token0,
        token1,
        score: std::cmp::min(reserve0, reserve1),
        liquidity: PoolLiquidity::V2 { reserve0, reserve1 },
        last_trade_timestamp: Some(u64::from(reserves.blockTimestampLast)),
    }))
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;
    use futures::StreamExt;

    use super::*;
    use crate::utils::get_test_provider;

    const FACTORY_ADDRESS: Address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");

    #[tokio::test]
    async fn test_fetch_all_pairs() {
        let provider = get_test_provider().await;
        let pairs = fetch_all_pairs(&provider, FACTORY_ADDRESS);

        let first_pairs = pairs.take(5).map(|x| x.unwrap()).collect::<Vec<_>>().await;

        let dummy: Vec<Address> = vec![
            address!("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc"),
            address!("0x3139ffc91b99aa94da8a2dc13f1fc36f9bdc98ee"),
            address!("0x12ede161c702d1494612d19f05992f43aa6a26fb"),
            address!("0xa478c2975ab1ea89e8196811f51a7b7ade33eb11"),
            address!("0x07f068ca326a469fc1d87d85d448990c8cba7df9"),
        ];

        assert_eq!(first_pairs, dummy)
    }

    #[tokio::test]
    async fn test_fetch_pair() {
        let provider = get_test_provider().await;
        let token_usdc = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
        let token_weth = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
        let pair = fetch_pair(&provider, FACTORY_ADDRESS, token_usdc, token_weth)
            .await
            .unwrap();

        assert_eq!(pair, address!("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc"));
    }
}
