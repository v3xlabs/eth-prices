use alloy::sol;

sol! {
    #[sol(rpc)]
    contract UniswapV3Pool {
         function slot0() public view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked);
         function token0() public view returns (address);
         function token1() public view returns (address);
    }
}
