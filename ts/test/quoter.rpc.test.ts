import { describe, expect, it } from "vitest";

import { createQuoter } from "../src/index";

const RPC_URL = "https://reth-ethereum.ithaca.xyz/rpc";
const BLOCK = 24692474n;
const USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC_WETH_V2_PAIR = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";

describe("wasm quoter real rpc", () => {
  it(
    "quotes USDC -> WETH at a fixed block",
    async () => {
      const quoter = await createQuoter({
        rpcUrl: RPC_URL,
        quoters: {
          uniswap_v2: [{ pair_address: USDC_WETH_V2_PAIR }],
        },
      });

      const route = quoter.computeRoute(USDC, WETH);
      const routeView = route.toJSON();
      expect(routeView.path.length).toBe(1);

      const amountOut = await quoter.getRate(USDC, WETH, "1000000", BLOCK);
      expect(amountOut).toBe("472461178704761");
    },
    30_000,
  );
});
