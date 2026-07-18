import { from as providerFrom } from "ox/Provider";
import { fromHttp } from "ox/RpcTransport";
import { describe, expect, it } from "vitest";

import { createNetworkContext } from "../src/network.js";
import { chainlinkQuoter } from "../src/quoters/chainlink/index.js";
import { curveDiscoverer } from "../src/quoters/curve/discovery.js";
import { curveQuoter } from "../src/quoters/curve/index.js";
import { erc4626Discoverer } from "../src/quoters/erc4626/discovery.js";
import { uniswapV2Discoverer } from "../src/quoters/uniswap_v2/discovery.js";
import { uniswapV2Quoter } from "../src/quoters/uniswap_v2/index.js";
import { uniswapV3Discoverer } from "../src/quoters/uniswap_v3/discovery.js";
import { createAutoRouter } from "../src/router/auto.js";

const RPC_URL = process.env.RPC_URL ?? "https://ethereum.reth.rs/rpc";
const BLOCK = 24_692_474n;
const USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const V2_PAIR = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const V2_FACTORY = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
const V3_FACTORY = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const ETH_USD_FEED = "0x5f4ec3df9cbd43714fe2740f5e3616155c5b8419";
const SDAI = "0x83F20F44975D03b1b09e64809B757c47f942BEeA";
const DAI = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
const CURVE_3POOL = "0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7";

const provider = providerFrom(fromHttp(RPC_URL));
const context = createNetworkContext({ 1: provider }, { blockNumber: BLOCK });

describe("live protocol integrations", () => {
  it("quotes a V2 pool at a pinned block", async () => {
    const quoter = uniswapV2Quoter({
      networkId: 1,
      pairAddress: V2_PAIR,
      token0: USDC,
      token1: WETH,
    });

    await expect(quoter.quote({ amountIn: 1_000_000n, direction: "forward", context }))
      .resolves.toBe(472_461_178_704_761n);
  }, 30_000);

  it("discovers competing V2 and V3 sources and returns a report", async () => {
    const result = await createAutoRouter({
      tokens: [USDC, WETH],
      context,
      discoverers: [
        erc4626Discoverer({ networkId: 1 }),
        uniswapV2Discoverer({ networkId: 1, factoryAddress: V2_FACTORY }),
        uniswapV3Discoverer({ networkId: 1, factoryAddress: V3_FACTORY }),
      ],
    });

    expect(result.report.discoverers).toHaveLength(3);
    expect(result.report.discoveredQuoters).toBeGreaterThanOrEqual(2);
    expect(result.router.quoters().some(quoter => quoter.identity.startsWith("uniswap_v2:"))).toBe(true);
    expect(result.router.quoters().some(quoter => quoter.identity.startsWith("uniswap_v3:"))).toBe(true);
    expect(result.router.compute(USDC, WETH).path).toHaveLength(1);
  }, 30_000);

  it("quotes a Curve StableSwap pool at a pinned block", async () => {
    const quoter = curveQuoter({
      networkId: 1,
      poolAddress: CURVE_3POOL,
      token0: DAI,
      token1: USDC,
      coinIndex0: 0,
      coinIndex1: 1,
      kind: "stableswap",
    });

    await expect(quoter.quote({ amountIn: 1_000_000_000_000_000_000n, direction: "forward", context }))
      .resolves.toBe(999_840n);
    await expect(quoter.quote({ amountIn: 1_000_000n, direction: "reverse", context }))
      .resolves.toBe(999_860_004_039_025_938n);
  }, 30_000);

  it("discovers the deepest Curve pool via the MetaRegistry", async () => {
    const result = await createAutoRouter({
      tokens: [DAI, USDC],
      context,
      discoverers: [curveDiscoverer({ networkId: 1 })],
    });

    expect(result.router.quoters().some(quoter => quoter.identity.startsWith("curve:"))).toBe(true);

    const amount = await result.router.quote(DAI, USDC, {
      amountIn: 1_000_000_000_000_000_000n,
      context,
    });

    expect(amount).toBeGreaterThan(900_000n);
    expect(amount).toBeLessThan(1_100_000n);
  }, 60_000);

  it("quotes a Chainlink feed at a pinned block", async () => {
    const quoter = chainlinkQuoter({
      networkId: 1,
      feedAddress: ETH_USD_FEED,
      token: WETH,
      quote: "fiat:usd",
      feedDecimals: 8,
      tokenDecimals: 18,
      quoteDecimals: 6,
    });
    const amount = await quoter.quote({ amountIn: 1_000_000_000_000_000_000n, direction: "forward", context });

    expect(amount).toBeGreaterThan(1_000_000_000n);
  }, 30_000);

  it("discovers and quotes an ERC-4626 vault", async () => {
    const result = await createAutoRouter({
      tokens: [SDAI, DAI],
      context,
      discoverers: [erc4626Discoverer({ networkId: 1 })],
    });
    const route = result.router.compute(SDAI, DAI);
    const amount = await result.router.quote(SDAI, DAI, {
      amountIn: 1_000_000_000_000_000_000n,
      context,
    });

    expect(route.path).toHaveLength(1);
    expect(amount).toBeGreaterThan(1_000_000_000_000_000_000n);
    expect(result.report.discoverers[0]?.failures).toHaveLength(1);
  }, 30_000);
});
