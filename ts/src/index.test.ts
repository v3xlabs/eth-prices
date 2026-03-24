import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test } from 'vitest'

import init, { EthPrices, eth_prices_version, initSync } from '../pkg/eth_prices.js'

const FIXED_GRAPH_TOML = `
[chains.mainnet]
chain_id = 1
rpc_url = "https://reth-ethereum.ithaca.xyz/rpc"
tokens = []

[chains.mainnet.quoters]
fixed = [
  { token_in = "fiat:usd", token_out = "native", fixed_rate = 2.0 }
]
erc4626 = []
`

test('loads web-target wasm bindings', () => {
    const currentDir = dirname(fileURLToPath(import.meta.url));
    const wasmPath = join(currentDir, '../pkg/eth_prices_bg.wasm');
    const wasmBytes = readFileSync(wasmPath);

    initSync({ module: wasmBytes });

    const version = eth_prices_version();
    expect(typeof version).toBe('string');
    expect(version).toBe('0.0.2');
})

test('web api can parse config and compute router quote', async () => {
    const currentDir = dirname(fileURLToPath(import.meta.url));
    const wasmPath = join(currentDir, '../pkg/eth_prices_bg.wasm');
    const wasmBytes = readFileSync(wasmPath);
    await init({ module_or_path: wasmBytes });

    const parsed = EthPrices.parseConfigToml(FIXED_GRAPH_TOML);
    expect(parsed.mainnet.chain_id).toBe(1);
    expect(parsed.mainnet.fixed_quoters).toBe(1);

    const client = await EthPrices.connect('https://reth-ethereum.ithaca.xyz/rpc');
    const graph = await client.graphFromToml(FIXED_GRAPH_TOML, 'mainnet');

    expect(graph.quoterCount()).toBe(1);

    const route = graph.compute('fiat:usd', 'native');
    const quoted = await route.quote(0, '1000000');

    expect(quoted).toBe('2000000');
})
