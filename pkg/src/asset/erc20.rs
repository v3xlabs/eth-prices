use std::collections::HashMap;

use alloy::{eips::BlockId, primitives::Address, sol};
use futures::future::join_all;

use crate::provider::RpcProvider;

sol! {
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    interface ERC20 {
        function balanceOf(address owner) external view returns (uint256);
        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
    }
}

const FALLBACK_DECIMALS: u8 = 18;

/// Fetches `decimals()` for every address concurrently, falling back to 18
/// where the optional ERC-20 method is missing or unreadable.
///
/// For strict, full metadata resolution of a single asset use
/// [`crate::asset::Asset::new`] instead.
pub async fn fetch_decimals(
    provider: &RpcProvider,
    addresses: &[Address],
    block: BlockId,
) -> HashMap<Address, u8> {
    join_all(addresses.iter().map(|address| {
        let provider = provider.clone();
        let address = *address;
        async move {
            let decimals = ERC20::new(address, &provider)
                .decimals()
                .block(block)
                .call()
                .await;
            (address, decimals.unwrap_or(FALLBACK_DECIMALS))
        }
    }))
    .await
    .into_iter()
    .collect()
}

/// Looks a token up in a [`fetch_decimals`] result, falling back to 18.
pub fn decimals_of(decimals: &HashMap<Address, u8>, token: Address) -> u8 {
    decimals.get(&token).copied().unwrap_or(FALLBACK_DECIMALS)
}
