import { readFile } from "node:fs/promises";

import { from as providerFrom } from "ox/Provider";
import { fromHttp } from "ox/RpcTransport";
import {
  createAutoRouter,
  createNetworkContext,
  curveDiscoverer,
  erc4626Discoverer,
  uniswapV2Discoverer,
  uniswapV3Discoverer,
} from "eth-prices";

const manifest = JSON.parse(await readFile(new URL("./cases.json", import.meta.url), "utf8"));
const rpcUrl = process.env.RPC_URL ?? "https://ethereum.reth.rs/rpc";
const iterations = Math.max(1, Number.parseInt(process.env.EVAL_ITERATIONS ?? "1000", 10));
if (!Number.isSafeInteger(iterations)) throw new Error("EVAL_ITERATIONS must be a positive integer");

const baseProvider = providerFrom(fromHttp(rpcUrl));
let rpcRequests = 0;
const provider = {
  request(parameters) {
    rpcRequests += 1;
    return baseProvider.request(parameters);
  },
};
const blockNumber = process.env.EVAL_BLOCK === undefined
  ? BigInt(await provider.request({ method: "eth_blockNumber" }))
  : BigInt(process.env.EVAL_BLOCK);
const context = createNetworkContext({ [manifest.networkId]: provider }, { blockNumber });
const runs = [];

const errorMessage = error => error instanceof Error ? error.message : String(error);
const nominalAmount = decimals => 10n ** BigInt(decimals);

for (const run of manifest.runs) {
  const discoverers = [];
  if (run.protocols.erc4626) discoverers.push(erc4626Discoverer({ networkId: manifest.networkId }));
  if (run.protocols.v2) {
    discoverers.push(uniswapV2Discoverer({
      networkId: manifest.networkId,
      factoryAddress: manifest.v2Factory,
    }));
  }
  if (run.protocols.v3) {
    discoverers.push(uniswapV3Discoverer({
      networkId: manifest.networkId,
      factoryAddress: manifest.v3Factory,
      fees: manifest.v3Fees,
    }));
  }
  if (run.protocols.curve) {
    discoverers.push(curveDiscoverer({
      networkId: manifest.networkId,
      metaRegistryAddress: manifest.curveMetaRegistry,
    }));
  }

  const requestsBefore = rpcRequests;
  const discoveryStartedAt = performance.now();
  let result;
  try {
    result = await createAutoRouter({
      tokens: run.tokens.map(name => manifest.assets[name].address),
      discoverers,
      context,
    });
  }
  catch (error) {
    runs.push({
      name: run.name,
      discoveryMs: performance.now() - discoveryStartedAt,
      discoveredQuoters: 0,
      discoveryRpcRequests: rpcRequests - requestsBefore,
      totalRpcRequests: rpcRequests - requestsBefore,
      quotes: [],
      error: errorMessage(error),
    });
    continue;
  }
  const discoveryMs = performance.now() - discoveryStartedAt;
  const discoveryRpcRequests = rpcRequests - requestsBefore;

  const quotes = [];
  for (const assetName of run.quotes) {
    const asset = manifest.assets[assetName];
    const outputAsset = manifest.assets.usdc;
    const amountIn = nominalAmount(asset.decimals);
    let route;
    try {
      route = result.router.compute(asset.address, outputAsset.address);
    }
    catch (error) {
      quotes.push({
        asset: assetName,
        outputAsset: "usdc",
        inputAmount: amountIn.toString(),
        sources: [],
        error: errorMessage(error),
      });
      continue;
    }

    let routeChecksum = 0;
    const routeStartedAt = process.hrtime.bigint();
    for (let index = 0; index < iterations; index += 1) {
      routeChecksum += result.router.compute(asset.address, outputAsset.address).path.length;
    }
    const routeElapsed = process.hrtime.bigint() - routeStartedAt;
    const routeComputeNs = Number(routeElapsed) / iterations;
    if (routeChecksum < 0) throw new Error("unreachable route checksum");
    const sources = route.path.map(step => step.quoter.identity);
    const quoteRequestsBefore = rpcRequests;
    const quoteStartedAt = performance.now();
    try {
      const outputAmount = await result.router.quote(asset.address, outputAsset.address, { amountIn, context });
      quotes.push({
        asset: assetName,
        outputAsset: "usdc",
        inputAmount: amountIn.toString(),
        outputAmount: outputAmount.toString(),
        routeComputeNs,
        quoteMs: performance.now() - quoteStartedAt,
        hops: route.path.length,
        sources,
        rpcRequests: rpcRequests - quoteRequestsBefore,
      });
    }
    catch (error) {
      quotes.push({
        asset: assetName,
        outputAsset: "usdc",
        inputAmount: amountIn.toString(),
        routeComputeNs,
        quoteMs: performance.now() - quoteStartedAt,
        hops: route.path.length,
        sources,
        rpcRequests: rpcRequests - quoteRequestsBefore,
        error: errorMessage(error),
      });
    }
  }

  runs.push({
    name: run.name,
    discoveryMs,
    discoveredQuoters: result.router.quoters().length,
    discoveryRpcRequests,
    totalRpcRequests: rpcRequests - requestsBefore,
    discoveryReport: {
      discoveredAssets: result.report.discoveredAssets,
      discoverers: result.report.discoverers.map(report => ({
        identity: report.identity,
        durationMs: report.durationMs,
        attempted: report.attempted,
        discovered: report.discovered,
        skipped: report.skipped,
        failures: report.failures.map(failure => ({ target: failure.target, message: failure.message })),
      })),
    },
    quotes,
  });
}

process.stdout.write(`${JSON.stringify({
  implementation: "typescript",
  blockNumber: Number(blockNumber),
  discoveryBlockPinned: true,
  iterations,
  runs,
})}\n`);
