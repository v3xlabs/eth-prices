use alloy::{
    primitives::{Address, address},
    sol,
};

/// Curve MetaRegistry on Ethereum mainnet, aggregating every pool registry.
pub const CURVE_META_REGISTRY_MAINNET: Address =
    address!("0xF98B45FA17DE75FB1aD0e7aFD971b0ca00e379fC");

sol! {
    #[sol(rpc)]
    contract CurveMetaRegistry {
        function find_pools_for_coins(address from, address to) external view returns (address[] memory);
        function get_coin_indices(address pool, address from, address to) external view returns (int128 i, int128 j, bool is_underlying);
        function get_balances(address pool) external view returns (uint256[8] memory balances);
    }
}
