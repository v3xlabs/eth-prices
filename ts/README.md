<h1 align="center">
    eth-prices
</h1>

<p align="center">
  A smol TypeScript package for fetching Ethereum asset prices directly from rpc.
</p>
<p align="center">
    <a href="https://www.npmjs.com/package/eth-prices"><img src="https://img.shields.io/npm/v/eth-prices?logo=npm&label=npm&color=cb3837&style=flat" alt="npm"></a>
    <a href="https://github.com/v3xlabs/eth-prices"><img src="https://img.shields.io/badge/Repository-v3xlabs/eth--prices-blue?style=flat" alt="Repository"></a>
    <a href="#"><img src="https://img.shields.io/badge/Status-In%20Development-blue?style=flat" alt="Status: In Development"></a>
    <a href="#"><img src="https://img.shields.io/badge/License-LGPL--3.0-hotpink?style=flat" alt="License: LGPL-3.0"></a>
</p>

> [!IMPORTANT]
> eth-prices aims to provide Ethereum price estimation only. It is not intended for, nor does it provide guarantees of, exchange rates.

`eth-prices` is ESM-only and requires Node.js 20 or newer.

## Quickstart

```sh
npm install eth-prices ox
```

```ts
import { from as providerFrom } from "ox/Provider";
import { fromHttp } from "ox/RpcTransport";
import {
  chainlinkQuoter,
  createNetworkContext,
  createRouter,
  uniswapV2Quoter,
} from "eth-prices";

const mainnet = 1;
const usdc = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const weth = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const usd = "fiat:usd";

// Define your data sources.
const router = createRouter([
  uniswapV2Quoter({
    networkId: mainnet,
    pairAddress: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
    token0: usdc,
    token1: weth,
  }),
  chainlinkQuoter({
    networkId: mainnet,
    feedAddress: "0x5f4ec3df9cbd43714fe2740f5e3616155c5b8419",
    token: weth,
    quote: usd,
    feedDecimals: 8,
    tokenDecimals: 18,
    quoteDecimals: 6,
  }),
]);

// Connect a provider and pin the quote to a block.
const rpcUrl = process.env.ETHEREUM_RPC_URL;

const provider = providerFrom(fromHttp(rpcUrl));
const context = createNetworkContext({ [mainnet]: provider }, {
  blockNumber: 24_692_474n,
});

// Compute the USDC -> WETH -> USD route.
const route = router.compute(usdc, usd);

// Quote 1 USDC. Inputs and outputs are bigint base units.
const amountOut = await router.quote(usdc, usd, {
  amountIn: 1_000_000n,
  context,
});

console.log(route.path.map(step => step.quoter.identity));
console.log(amountOut); // USD with 6 decimal places
```

`createNetworkContext` accepts any EIP-1193-style provider. `ox` is used only to create the HTTP provider in this example. Omit `blockNumber` to quote the latest block.

## Data Sources

Currently supported data sources include:

- **Uniswap V2**: `uniswapV2Quoter` for Uniswap V2-compatible pairs.
- **Uniswap V3**: `uniswapV3Quoter` for Uniswap V3-compatible pools.
- **ERC-4626**: `erc4626Quoter` for ERC-4626-compatible vaults such as Morpho and Aave.
- **Fixed**: `fixedQuoter` for static rates, such as WETH to ETH or USDC to USD.
- **European Central Bank**: `createEcbRateSource` and `ecbQuoter` for EUR-based fiat exchange rates.
- **Chainlink**: `chainlinkQuoter` for Chainlink Price Feeds.

## Examples

This package supports both explicit routing and automatic discovery:

- **Explicit router**: construct quoters directly when pool, vault, or feed addresses are known, as shown in the [Quickstart](#quickstart).
- **Auto router**: discover compatible vaults and pools from selected deployments.

```ts
import {
  createAutoRouter,
  erc4626Discoverer,
  uniswapV2Discoverer,
  uniswapV3Discoverer,
} from "eth-prices";

const { router, report } = await createAutoRouter({
  tokens: [usdc, weth],
  context,
  discoverers: [
    erc4626Discoverer({ networkId: mainnet }),
    uniswapV2Discoverer({
      networkId: mainnet,
      factoryAddress: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
    }),
    uniswapV3Discoverer({
      networkId: mainnet,
      factoryAddress: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    }),
  ],
});

console.log(router.quoters());
console.log(report.discoverers);
```

## License

[LGPL-3.0](../LICENSE)
