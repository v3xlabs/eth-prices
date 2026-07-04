# eth-prices

Pure TypeScript price routing and quoting for EVM assets.

`eth-prices` builds a graph of independent price sources and computes confidence-weighted routes between assets. It supports fixed rates, Uniswap V2-compatible pairs, Uniswap V3-compatible pools, ERC-4626 vaults, Chainlink feeds, and European Central Bank fiat rates.

The package is ESM-only and requires Node.js 20 or newer.

## Install

```sh
pnpm add eth-prices ox
```

## Explicit router

Construct quoters directly when pool and feed addresses are known. Explicit block numbers make quotes reproducible; omit `blockNumber` to query the latest block.

```ts
import { from as providerFrom } from "ox/Provider";
import { fromHttp } from "ox/RpcTransport";
import {
  createNetworkContext,
  createRouter,
  uniswapV2Quoter,
} from "eth-prices";

const provider = providerFrom(fromHttp(process.env.RPC_URL));
const context = createNetworkContext(
  { 1: provider },
  { blockNumber: 24_692_474n },
);

const USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

const router = createRouter([
  uniswapV2Quoter({
    networkId: 1,
    pairAddress: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
    token0: USDC,
    token1: WETH,
  }),
]);

const route = router.compute(USDC, WETH);
const amountOut = await router.quote(USDC, WETH, {
  amountIn: 1_000_000n,
  context,
});
```

## Automatic discovery

Automatic routing is protocol- and chain-neutral. Supply the discoverers and deployment addresses that should be queried. Multiple compatible deployments can participate in the same router.

Discoverers run in order. Put ERC-4626 discovery before pool discovery when vault underlyings should be included in subsequent pool searches.

```ts
import {
  createAutoRouter,
  createNetworkContext,
  erc4626Discoverer,
  uniswapV2Discoverer,
  uniswapV3Discoverer,
} from "eth-prices";

const context = createNetworkContext({ 1: provider });

const { router, report } = await createAutoRouter({
  tokens: [USDC, WETH],
  context,
  discoverers: [
    erc4626Discoverer({ networkId: 1 }),
    uniswapV2Discoverer({
      networkId: 1,
      factoryAddress: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
    }),
    uniswapV2Discoverer({
      networkId: 1,
      identity: "another_v2_deployment",
      factoryAddress: "0x0000000000000000000000000000000000000001",
    }),
    uniswapV3Discoverer({
      networkId: 1,
      factoryAddress: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    }),
  ],
});

console.log(report.discoverers);
```

Discovery is best-effort. A failed pool or vault probe is recorded in `report` without discarding successful discoveries from other targets or adapters. Inspect the report before relying on a router when provider completeness matters.

## Routing policy

The router retains parallel quote sources. Each edge costs `101 - min(confidence, 100)`, matching the Rust crate's routing policy. Route computation minimizes total edge cost, so several high-confidence hops can beat one low-confidence direct source.

Automatically discovered sources receive confidence from liquidity:

- ERC-4626: `50`
- V2-compatible pools: `min(100, min(reserve0, reserve1) / 1_000_000_000)`
- V3-compatible pools: `min(100, liquidity / 10_000_000_000_000_000)`

These are spot-price quoters. V2 and V3 quotes do not estimate executable swap output, fees, price impact, or slippage.

## Asset identifiers

EVM addresses are validated and canonicalized. Address casing therefore does not create separate graph nodes. Other identifiers remain exact strings:

- `native`
- `fiat:usd`
- custom identifiers such as `commodity:gold`

All amounts, block numbers, timestamps, and rates use `bigint`.

## Errors

Library errors are instances of `EthPricesError` and include a stable `code`. Discovery adapters report target-level failures instead of throwing them from `createAutoRouter`; invalid top-level configuration still throws.
