import { readFileSync } from "node:fs";
// @ts-ignore - wasm-bindgen generated file
import { __wbg_set_wasm } from "../pkg/eth_prices_bg.js";
// @ts-ignore - wasm-bindgen generated file
import * as bg from "../pkg/eth_prices_bg.js";

const wasmBytes = readFileSync(new URL("../pkg/eth_prices_bg.wasm", import.meta.url));
const wasmModule = new WebAssembly.Module(wasmBytes);
const wasmInstance = new WebAssembly.Instance(wasmModule, {
    "./eth_prices_bg.js": bg,
});

__wbg_set_wasm(wasmInstance.exports);
(wasmInstance.exports as Record<string, Function>).__wbindgen_start();

// @ts-ignore - re-export from wasm-bindgen glue (types provided via package.json "types" field)
export { Engine, Route, createEngine } from "../pkg/eth_prices_bg.js";
