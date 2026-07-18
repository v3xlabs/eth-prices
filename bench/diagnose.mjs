import { readFile } from "node:fs/promises";

import { from as providerFrom } from "ox/Provider";
import { fromHttp } from "ox/RpcTransport";
import {
  createAutoRouter,
  createNetworkContext,
  erc4626Discoverer,
  uniswapV2Discoverer,
  uniswapV3Discoverer,
} from "eth-prices";

import { formatRoute } from "./report.mjs";

const manifest = JSON.parse(await readFile(new URL("./cases.json", import.meta.url), "utf8"));
const latest = await readFile(new URL("../target/evals/latest.json", import.meta.url), "utf8")
  .then(JSON.parse)
  .catch(() => undefined);
const rpcUrl = process.env.RPC_URL ?? "https://ethereum.reth.rs/rpc";
const provider = providerFrom(fromHttp(rpcUrl));
const blockNumber = latest === undefined
  ? BigInt(await provider.request({ method: "eth_blockNumber" }))
  : BigInt(latest.blockNumber);
const context = createNetworkContext({ [manifest.networkId]: provider }, { blockNumber });
const requested = process.argv.slice(2);
const assetNames = requested.length > 0
  ? requested
  : [...new Set(manifest.runs.flatMap(run => run.quotes))];

for (const name of assetNames) {
  if (manifest.assets[name] === undefined) {
    console.error(`unknown asset ${name}; expected one of: ${Object.keys(manifest.assets).join(", ")}`);
    process.exit(2);
  }
}

const addressToName = new Map(Object.entries(manifest.assets)
  .map(([name, asset]) => [asset.address.toLowerCase(), name]));
const nameOf = address => addressToName.get(address.toLowerCase()) ?? address.slice(0, 10);
const usdc = manifest.assets.usdc.address;
const run = manifest.runs[0];

console.error(`discovering at block ${blockNumber}…`);
const { router } = await createAutoRouter({
  tokens: run.tokens.map(name => manifest.assets[name].address),
  discoverers: [
    erc4626Discoverer({ networkId: manifest.networkId }),
    uniswapV2Discoverer({ networkId: manifest.networkId, factoryAddress: manifest.v2Factory }),
    uniswapV3Discoverer({ networkId: manifest.networkId, factoryAddress: manifest.v3Factory, fees: manifest.v3Fees }),
  ],
  context,
});

for (const assetName of assetNames) {
  const asset = manifest.assets[assetName];
  if (asset.address.toLowerCase() === usdc.toLowerCase()) continue;
  const consensus = latest?.referenceStatistics?.[assetName]?.median;
  console.log(`\n${assetName} — consensus ${consensus === undefined ? "unavailable" : `$${consensus}`}`);
  try {
    const route = router.compute(asset.address, usdc);
    console.log(`  chosen route: ${formatRoute(route.path.map(step => step.quoter.identity))}`);
  } catch (error) {
    console.log(`  chosen route: none (${error instanceof Error ? error.message : String(error)})`);
  }

  const adjacent = router.quoters()
    .filter(quoter => quoter.assets.some(a => a.toLowerCase() === asset.address.toLowerCase()))
    .sort((left, right) => right.confidence - left.confidence);
  for (const quoter of adjacent) {
    const other = quoter.assets.find(a => a.toLowerCase() !== asset.address.toLowerCase());
    const direction = quoter.assets[0].toLowerCase() === asset.address.toLowerCase() ? "forward" : "reverse";
    const amountIn = 10n ** BigInt(asset.decimals);
    let priceUsd;
    let note = "";
    try {
      const amountOut = await quoter.quote({ amountIn, direction, context });
      if (other.toLowerCase() === usdc.toLowerCase()) {
        priceUsd = Number(amountOut) / 10 ** manifest.assets.usdc.decimals;
      } else {
        const usdOut = await router.quote(other, usdc, { amountIn: amountOut, context });
        priceUsd = Number(usdOut) / 10 ** manifest.assets.usdc.decimals;
        note = " (via router)";
      }
    } catch (error) {
      note = ` quote failed: ${error instanceof Error ? error.message : String(error)}`;
    }
    const errorPercent = priceUsd !== undefined && consensus !== undefined
      ? `  err ${(Math.abs(priceUsd - consensus) / consensus * 100).toFixed(3)}%`
      : "";
    console.log(
      `  conf ${String(quoter.confidence).padStart(3)}  vs ${nameOf(other).padEnd(8)}`
      + ` ${priceUsd === undefined ? "" : `$${priceUsd.toPrecision(6)}`}${errorPercent}${note}`
      + `  ${quoter.identity}`,
    );
  }
}
