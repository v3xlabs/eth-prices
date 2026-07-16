import { execFile as execFileCallback } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { promisify } from "node:util";

import { implementationScore, parityScore } from "./score.mjs";
import { fetchUsdSources } from "./sources.mjs";

const execFile = promisify(execFileCallback);
const root = new URL("../", import.meta.url);
const manifest = JSON.parse(await readFile(new URL("./cases.json", import.meta.url), "utf8"));
const rpcUrl = process.env.RPC_URL ?? "https://ethereum.reth.rs/rpc";
const iterations = process.env.EVAL_ITERATIONS ?? "1000";
const shouldSkipTypeScriptBuild = process.env.EVAL_SKIP_TS_BUILD === "true";
const rustBinary = process.env.EVAL_RUST_BINARY;

async function latestBlock() {
  if (process.env.EVAL_BLOCK !== undefined) return Number(process.env.EVAL_BLOCK);
  const response = await fetch(rpcUrl, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "eth_blockNumber", params: [] }),
  });
  if (!response.ok) throw new Error(`RPC returned HTTP ${response.status}`);
  const payload = await response.json();
  if (payload.error !== undefined) throw new Error(`RPC error: ${payload.error.message ?? JSON.stringify(payload.error)}`);
  return Number(BigInt(payload.result));
}

async function command(file, args, environment = {}, parseJson = true) {
  const { stdout, stderr } = await execFile(file, args, {
    cwd: root,
    env: { ...process.env, ...environment },
    maxBuffer: 20 * 1024 * 1024,
  });
  if (stderr.trim() !== "") process.stderr.write(stderr);
  return parseJson ? JSON.parse(stdout) : undefined;
}

function median(values) {
  if (values.length === 0) return undefined;
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

function percentageError(actual, expected) {
  return Math.abs(actual - expected) / expected * 100;
}

function quoteIndex(output) {
  return new Map(output.runs.flatMap(run => run.quotes.map(quote => [`${run.name}:${quote.asset}`, { run, quote }])));
}

const blockNumber = await latestBlock();
const references = {};
await Promise.all(Object.entries(manifest.assets).map(async ([name, asset]) => {
  if (asset.reference === undefined) return;
  references[name] = await fetchUsdSources(asset.reference, {
    timeoutMs: Number(process.env.EVAL_SOURCE_TIMEOUT_MS ?? "10000"),
  });
}));

if (!shouldSkipTypeScriptBuild) await command("pnpm", ["--dir", "ts", "build"], {}, false);
const environment = { EVAL_BLOCK: String(blockNumber), EVAL_ITERATIONS: iterations, RPC_URL: rpcUrl };
const rust = rustBinary === undefined
  ? await command("cargo", ["run", "--quiet", "--release", "-p", "eth-prices-bench"], environment)
  : await command(rustBinary, [], environment);
const typescript = await command("node", ["bench/typescript.mjs"], environment);
const rustQuotes = quoteIndex(rust);
const typescriptQuotes = quoteIndex(typescript);
const comparisons = [];

for (const key of new Set([...rustQuotes.keys(), ...typescriptQuotes.keys()])) {
  const rustEntry = rustQuotes.get(key);
  const typescriptEntry = typescriptQuotes.get(key);
  const assetName = rustEntry?.quote.asset ?? typescriptEntry?.quote.asset;
  const reference = references[assetName];
  const consensusUsd = median(reference?.records.map(record => record.priceUsd) ?? []);
  const rustUsd = rustEntry?.quote.outputAmount === undefined
    ? undefined
    : Number(BigInt(rustEntry.quote.outputAmount)) / 10 ** manifest.assets.usdc.decimals;
  const typescriptUsd = typescriptEntry?.quote.outputAmount === undefined
    ? undefined
    : Number(BigInt(typescriptEntry.quote.outputAmount)) / 10 ** manifest.assets.usdc.decimals;
  const sourceErrors = (reference?.records ?? []).map(record => ({
    source: record.source,
    priceUsd: record.priceUsd,
    rustErrorPercent: rustUsd === undefined ? undefined : percentageError(rustUsd, record.priceUsd),
    typescriptErrorPercent: typescriptUsd === undefined ? undefined : percentageError(typescriptUsd, record.priceUsd),
  }));
  comparisons.push({
    case: key,
    asset: assetName,
    consensusUsd,
    rustUsd,
    typescriptUsd,
    rustErrorPercent: rustUsd === undefined || consensusUsd === undefined ? undefined : percentageError(rustUsd, consensusUsd),
    typescriptErrorPercent: typescriptUsd === undefined || consensusUsd === undefined ? undefined : percentageError(typescriptUsd, consensusUsd),
    crossLanguageDifferencePercent: rustUsd === undefined || typescriptUsd === undefined
      ? undefined
      : percentageError(rustUsd, typescriptUsd),
    sourceErrors,
    rustSources: rustEntry?.quote.sources ?? [],
    typescriptSources: typescriptEntry?.quote.sources ?? [],
    rustError: rustEntry?.quote.error,
    typescriptError: typescriptEntry?.quote.error,
  });
}

const scores = {
  rust: implementationScore(comparisons, "rustUsd", "rustErrorPercent"),
  typescript: implementationScore(comparisons, "typescriptUsd", "typescriptErrorPercent"),
  parity: parityScore(comparisons),
};

const generatedAt = new Date();
const report = {
  generatedAt: generatedAt.toISOString(),
  blockNumber,
  rpcUrl,
  references,
  implementations: { rust, typescript },
  scores,
  comparisons,
};
const outputDirectory = new URL("../target/evals/", import.meta.url);
await mkdir(outputDirectory, { recursive: true });
const timestamp = generatedAt.toISOString().replaceAll(":", "-");
const outputUrl = new URL(`${timestamp}.json`, outputDirectory);
await writeFile(outputUrl, `${JSON.stringify(report, undefined, 2)}\n`);
await writeFile(new URL("latest.json", outputDirectory), `${JSON.stringify(report, undefined, 2)}\n`);

console.log(`Block: ${blockNumber}`);
console.log(`Report: ${outputUrl.pathname}`);
console.table(Object.entries({ rust: scores.rust, typescript: scores.typescript }).map(([implementation, score]) => ({
  implementation,
  priceScore: score.priceScore?.toFixed(3) ?? "n/a",
  mape: score.meanAbsolutePercentageError?.toFixed(3) ?? "n/a",
  pricedRoutes: `${score.pricedRoutes}/${score.expectedPricedRoutes}`,
  routeCoverage: `${score.routeCoveragePercent.toFixed(1)}%`,
})));
console.table(comparisons.map(comparison => ({
  case: comparison.case,
  reference: comparison.consensusUsd?.toFixed(2) ?? "n/a",
  rust: comparison.rustUsd?.toFixed(2) ?? "error",
  rustError: comparison.rustErrorPercent?.toFixed(3) ?? "n/a",
  typescript: comparison.typescriptUsd?.toFixed(2) ?? "error",
  typescriptError: comparison.typescriptErrorPercent?.toFixed(3) ?? "n/a",
})));

for (const [asset, result] of Object.entries(references)) {
  for (const error of result.errors) console.warn(`${asset}/${error.source}: ${error.error}`);
}
