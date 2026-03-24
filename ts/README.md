<h1 align="center">
    eth-prices
</h1>

<p align="center">
  Fast, WASM-powered price routing and quoting for Ethereum
</p>

<p align="center">
    <a href="https://github.com/v3xlabs/eth-prices"><img src="https://img.shields.io/badge/Repository-v3xlabs/eth--prices-blue?style=flat" alt="Repository"></a>
    <a href="#"><img src="https://img.shields.io/badge/License-LGPL--3.0-hotpink?style=flat" alt="License: LGPL-3.0"></a>
</p>

---

## Quickstart

```bash
pnpm add eth-prices
```

```ts
import { createEngine } from "eth-prices";

const WETH = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

const engine = await createEngine({
  rpcUrl: "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
  quoters: {
    uniswap_v3: [
      { token_in: WETH, token_out: USDC, fee: 500 }
    ],
    fixed: [
      { token_in: USDC, token_out: "fiat:usd", fixed_rate: 1 }
    ]
  },
});

// Compute the route once and hold onto it
const route = engine.computeRoute(WETH, "fiat:usd");

// Quote using the route — reuse it as many times as needed
const amountOut = await engine.quoteRoute(route, "1000000000000000000");
console.log(`Amount out: ${amountOut}`);
```

## Overview

`eth-prices` provides high-performance JS bindings for the `eth-prices` Rust engine. It allows you to build complex price routing graphs combining multiple DEX protocols and custom quoters, all executing with the efficiency of WebAssembly.
