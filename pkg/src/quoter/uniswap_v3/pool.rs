use alloy::sol;

sol! {
    #[sol(rpc)]
    contract UniswapV3Pool {
         function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked);
         function token0() external view returns (address);
         function token1() external view returns (address);
         function liquidity() external view returns (uint128 liquidity);
         function observations(uint256 index) external view returns (uint32 blockTimestamp, int56 tickCumulative, uint160 secondsPerLiquidityCumulativeX128, bool initialized);
    }
}
