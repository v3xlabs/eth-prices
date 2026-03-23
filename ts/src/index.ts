import {init_panic_hook, quoteUniswapV3} from "../pkg/eth_prices.js";

init_panic_hook();

const x = await quoteUniswapV3({});
