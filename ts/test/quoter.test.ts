import { describe, expect, it } from "vitest";

import { createEngine } from "../src/index";

declare global {
  namespace NodeJS {
    interface ProcessEnv {
      RPC_URL: string;
    }
  }
}

const RPC_URL = process.env.RPC_URL;
const BLOCK = 24692474n;
const USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const EURC = "0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c";
const WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const XAUT = "0x68749665FF8D2d112Fa859AA293F07A622782F38";
const KPK_VAULT_EURC = "0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1";
const USDC_WETH_V2_PAIR = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const XAUT_USDT_V3_POOL = "0x6546055f46e866a4B9a4A13e81273e3152BAE5dA";

describe("wasm quoter real rpc", () => {
  it(
    "quotes USDC -> WETH at a fixed block",
    async () => {
      const quoter = await createEngine({
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

  it(
    "quotes xAUT -> USDC through a two-hop route",
    async () => {
      const quoter = await createEngine({
        rpcUrl: RPC_URL,
        quoters: {
          uniswap_v3: [
            { pool_address: XAUT_USDT_V3_POOL },
            { token_in: USDT, token_out: USDC, fee: 500 },
          ],
        },
      });

      const route = quoter.computeRoute(XAUT, USDC);
      const routeView = route.toJSON();

      expect(routeView.path.map((step) => step.direction)).toEqual(["Forward", "Reverse"]);
      expect(routeView.path.map((step) => step.quoterId)).toEqual([
        "uniswap_v3:0x6546055f46e866a4B9a4A13e81273e3152BAE5dA",
        "uniswap_v3:0x7858E59e0C01EA06Df3aF3D20aC7B0003275D4Bf",
      ]);

      const amountOut = await quoter.quoteRoute(route, "1000000", BLOCK);
      expect(amountOut).toBe("4582921520");
    },
    30_000,
  );

  it(
    "quotes xAUT -> fiat:usd through a three-hop route",
    async () => {
      const engine = await createEngine({
        rpcUrl: RPC_URL,
        quoters: {
          fixed: [{ token_in: USDC, token_out: "fiat:usd", fixed_rate: 1 }],
          uniswap_v3: [
            { pool_address: XAUT_USDT_V3_POOL },
            { token_in: USDT, token_out: USDC, fee: 500 },
          ],
        },
      });

      const route = engine.computeRoute(XAUT, "fiat:usd");
      const routeView = route.toJSON();

      expect(routeView.path.map((step) => step.direction)).toEqual([
        "Forward",
        "Reverse",
        "Forward",
      ]);
      expect(routeView.path).toHaveLength(3);

      const amountOut = await engine.quoteRoute(route, "1000000", BLOCK);
      expect(amountOut).toBe("4582921520");
    },
    30_000,
  );

  it(
    "quotes kpk_EURC_Yield -> fiat:usd through vault and EURC/USDC route",
    async () => {
      const engine = await createEngine({
        rpcUrl: RPC_URL,
        quoters: {
          fixed: [{ token_in: USDC, token_out: "fiat:usd", fixed_rate: 1 }],
          uniswap_v3: [{ token_in: EURC, token_out: USDC, fee: 500 }],
          erc4626: [KPK_VAULT_EURC],
        },
      });

      const route = engine.computeRoute(KPK_VAULT_EURC, "fiat:usd");
      const routeView = route.toJSON();

      expect(routeView.path.map((step) => step.quoterId)).toEqual([
        "erc4626:0x0c6aec603d48eBf1cECc7b247a2c3DA08b398DC1",
        "uniswap_v3:0x95DBB3C7546F22BCE375900AbFdd64a4E5bD73d6",
        "fixed:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48:fiat:usd",
      ]);
      expect(routeView.path.map((step) => step.direction)).toEqual([
        "Forward",
        "Forward",
        "Forward",
      ]);

      const amountOut = await engine.quoteRoute(route, "1000000000000000000", BLOCK);
      expect(amountOut).toBe("1175390");
    },
    30_000,
  );
});
