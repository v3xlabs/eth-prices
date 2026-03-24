import wasmInit, {
  createEngine as wasmCreateEngine,
  initSync,
} from "./pkg/eth_prices.js";

export * from "./pkg/eth_prices.js";

let initPromise;

export async function init() {
  if (!initPromise) {
    initPromise = (async () => {
      if (typeof process !== "undefined" && process.versions?.node) {
        const { readFile } = await import("node:fs/promises");
        const wasmUrl = new URL("./pkg/eth_prices_bg.wasm", import.meta.url);
        const wasmBytes = await readFile(wasmUrl);
        return wasmInit({ module_or_path: wasmBytes });
      }

      return wasmInit();
    })();
  }

  return initPromise;
}

export async function createEngine(config) {
  await init();
  return wasmCreateEngine(config);
}

export { initSync };
