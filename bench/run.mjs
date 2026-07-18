import { execFile as execFileCallback } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { parseArgs, promisify } from "node:util";

import { renderReport } from "./report.mjs";
import {
  assessPrice,
  implementationScore,
  parityScore,
  percentageError,
  referenceStatistics,
} from "./score.mjs";
import { fetchUsdSources } from "./sources.mjs";

const USAGE = `Usage: node bench/run.mjs [options]

  --impl <rust|ts|both>   implementations to evaluate (default: both)
  --refs <latest|path>    reuse references and block from a previous report
                          instead of refetching the external price APIs
  --baseline <latest|path> report to diff against (default: latest.json)
  --block <number>        pin the evaluation block (default: snapshot block,
                          then EVAL_BLOCK, then the chain head)
  --iterations <number>   route-computation iterations (default: 1000)
  --json                  print the full report JSON to stdout
  --no-color              disable ANSI colors
  --help                  show this message

Environment: RPC_URL, EVAL_BLOCK, EVAL_ITERATIONS, EVAL_SOURCE_TIMEOUT_MS,
EVAL_SKIP_TS_BUILD, EVAL_RUST_BINARY.`;

const { values: options } = parseArgs({
  options: {
    impl: { type: "string", default: "both" },
    refs: { type: "string" },
    baseline: { type: "string" },
    block: { type: "string" },
    iterations: { type: "string" },
    json: { type: "boolean", default: false },
    "no-color": { type: "boolean", default: false },
    help: { type: "boolean", default: false },
  },
});

if (options.help) {
  console.log(USAGE);
  process.exit(0);
}

const IMPLEMENTATIONS = { rust: ["rust"], ts: ["typescript"], typescript: ["typescript"], both: ["rust", "typescript"] };
const implementationsRun = IMPLEMENTATIONS[options.impl];
if (implementationsRun === undefined) {
  console.error(`unknown --impl ${options.impl}; expected rust, ts, or both`);
  process.exit(2);
}

const root = new URL("../", import.meta.url);
const outputDirectory = new URL("./target/evals/", root);
const manifest = JSON.parse(await readFile(new URL("./cases.json", import.meta.url), "utf8"));
const rpcUrl = process.env.RPC_URL ?? "https://ethereum.reth.rs/rpc";
const iterations = options.iterations ?? process.env.EVAL_ITERATIONS ?? "1000";
const execFile = promisify(execFileCallback);
const progress = message => process.stderr.write(`${message}\n`);

function resolveReportUrl(value) {
  return value === "latest" ? new URL("latest.json", outputDirectory) : new URL(value, `file://${process.cwd()}/`);
}

async function readReport(url) {
  try {
    return JSON.parse(await readFile(url, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return undefined;
    throw error;
  }
}

async function latestBlock() {
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

const snapshot = options.refs === undefined ? undefined : await readReport(resolveReportUrl(options.refs));
if (options.refs !== undefined && snapshot === undefined) {
  console.error(`--refs ${options.refs}: report not found`);
  process.exit(2);
}

const blockNumber = options.block !== undefined
  ? Number(options.block)
  : snapshot?.blockNumber
    ?? (process.env.EVAL_BLOCK !== undefined ? Number(process.env.EVAL_BLOCK) : await latestBlock());

let references;
if (snapshot === undefined) {
  progress(`fetching references for block ${blockNumber}…`);
  references = {};
  await Promise.all(Object.entries(manifest.assets).map(async ([name, asset]) => {
    if (asset.reference === undefined) return;
    references[name] = await fetchUsdSources(asset.reference, {
      timeoutMs: Number(process.env.EVAL_SOURCE_TIMEOUT_MS ?? "10000"),
    });
  }));
} else {
  references = snapshot.references;
  progress(`reusing references from ${options.refs} (block ${blockNumber})`);
}

const statisticsByAsset = Object.fromEntries(Object.entries(references)
  .map(([asset, result]) => [asset, referenceStatistics(result.records)]));

const environment = { EVAL_BLOCK: String(blockNumber), EVAL_ITERATIONS: iterations, RPC_URL: rpcUrl };
const outputs = {};

async function runImplementation(name, task) {
  progress(`running ${name} autorouter…`);
  try {
    outputs[name] = await task();
  } catch (error) {
    console.error(`${name} runner failed: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

if (implementationsRun.includes("rust")) {
  await runImplementation("rust", () => process.env.EVAL_RUST_BINARY === undefined
    ? command("cargo", ["run", "--quiet", "--release", "-p", "eth-prices-bench"], environment)
    : command(process.env.EVAL_RUST_BINARY, [], environment));
}
if (implementationsRun.includes("typescript")) {
  if (process.env.EVAL_SKIP_TS_BUILD !== "true") await command("pnpm", ["--dir", "ts", "build"], {}, false);
  await runImplementation("typescript", () => command("node", ["bench/typescript.mjs"], environment));
}

function quoteIndex(output) {
  return new Map(output.runs.flatMap(run => run.quotes.map(quote => [`${run.name}:${quote.asset}`, quote])));
}

function usdAmount(quote) {
  if (quote?.outputAmount === undefined) return undefined;
  return Number(BigInt(quote.outputAmount)) / 10 ** manifest.assets[quote.outputAsset].decimals;
}

const indexes = Object.fromEntries(Object.entries(outputs).map(([name, output]) => [name, quoteIndex(output)]));
const manifestKeys = manifest.runs.flatMap(run => run.quotes.map(asset => `${run.name}:${asset}`));
const caseKeys = [...new Set([...manifestKeys, ...Object.values(indexes).flatMap(index => [...index.keys()])])];

const comparisons = caseKeys.map(key => {
  const quotes = Object.fromEntries(implementationsRun
    .map(name => [name, indexes[name]?.get(key)])
    .filter(([, quote]) => quote !== undefined));
  const asset = Object.values(quotes)[0]?.asset ?? key.split(":").at(-1);
  const reference = statisticsByAsset[asset];
  const implementations = Object.fromEntries(Object.entries(quotes).map(([name, quote]) => {
    const priceUsd = usdAmount(quote);
    return [name, {
      ...priceUsd === undefined ? {} : { priceUsd },
      ...quote.outputAmount === undefined ? {} : { outputAmount: quote.outputAmount },
      assessment: assessPrice(priceUsd, reference),
      sources: quote.sources ?? [],
      ...quote.hops === undefined ? {} : { hops: quote.hops },
      ...quote.routeComputeNs === undefined ? {} : { routeComputeNs: quote.routeComputeNs },
      ...quote.quoteMs === undefined ? {} : { quoteMs: quote.quoteMs },
      ...quote.rpcRequests === undefined ? {} : { rpcRequests: quote.rpcRequests },
      ...quote.error === undefined ? {} : { error: quote.error },
    }];
  }));
  const priced = implementationsRun
    .map(name => implementations[name]?.priceUsd)
    .filter(price => price !== undefined);

  return {
    case: key,
    asset,
    reference,
    implementations,
    ...priced.length === 2 ? { crossDifferencePercent: percentageError(priced[0], priced[1]) } : {},
  };
});

const scores = Object.fromEntries(implementationsRun.map(name => [name, implementationScore(comparisons, name)]));
if (implementationsRun.length === 2) scores.parity = parityScore(comparisons);

const baselineUrl = resolveReportUrl(options.baseline ?? "latest");
const baselineReport = await readReport(baselineUrl);

function baselineDelta(baseline) {
  if (baseline?.comparisons?.[0]?.implementations === undefined) return undefined;
  const pair = (from, to) => ({ from, to });
  const scoreDeltas = implementationsRun
    .filter(name => baseline.scores?.[name] !== undefined)
    .map(name => ({
      implementation: name,
      priceScore: pair(baseline.scores[name].priceScore, scores[name].priceScore),
      meanAbsolutePercentageError: pair(
        baseline.scores[name].meanAbsolutePercentageError,
        scores[name].meanAbsolutePercentageError,
      ),
      routeCoveragePercent: pair(baseline.scores[name].routeCoveragePercent, scores[name].routeCoveragePercent),
    }));
  const baselineCases = new Map(baseline.comparisons.map(comparison => [comparison.case, comparison]));
  const caseDeltas = [];
  for (const comparison of comparisons) {
    for (const name of implementationsRun) {
      const from = baselineCases.get(comparison.case)?.implementations?.[name]?.assessment?.errorPercent;
      const to = comparison.implementations[name]?.assessment?.errorPercent;
      if (from === undefined && to === undefined) continue;
      if (from !== undefined && to !== undefined && Math.abs(from - to) < 0.0005) continue;
      caseDeltas.push({ case: comparison.case, implementation: name, from, to });
    }
  }
  caseDeltas.sort((left, right) => {
    const magnitude = entry => entry.to === undefined || entry.from === undefined
      ? Number.POSITIVE_INFINITY
      : Math.abs(entry.to - entry.from);
    return magnitude(right) - magnitude(left);
  });
  if (scoreDeltas.length === 0 && caseDeltas.length === 0) return undefined;

  return {
    path: baselineUrl.pathname,
    generatedAt: baseline.generatedAt,
    blockNumber: baseline.blockNumber,
    scores: scoreDeltas,
    caseDeltas,
  };
}

const generatedAt = new Date();
const timestamp = generatedAt.toISOString().replaceAll(":", "-");
const reportUrl = new URL(`${timestamp}.json`, outputDirectory);
const report = {
  generatedAt: generatedAt.toISOString(),
  blockNumber,
  rpcUrl,
  iterations: Number(iterations),
  implementationsRun,
  ...options.refs === undefined ? {} : { referencesFrom: resolveReportUrl(options.refs).pathname },
  reportPath: reportUrl.pathname,
  references,
  referenceStatistics: statisticsByAsset,
  implementations: outputs,
  scores,
  comparisons,
  baseline: baselineDelta(baselineReport),
};

await mkdir(outputDirectory, { recursive: true });
const serialized = `${JSON.stringify(report, undefined, 2)}\n`;
await writeFile(reportUrl, serialized);
await writeFile(new URL("latest.json", outputDirectory), serialized);

if (options.json) {
  process.stdout.write(serialized);
} else {
  const colors = !options["no-color"]
    && process.env.NO_COLOR === undefined
    && (process.stdout.isTTY === true || process.env.FORCE_COLOR !== undefined);
  console.log(renderReport(report, { colors }));
}
