import { canonicalizeAddress, type EvmAddress } from "../asset.js";
import { EthPricesError } from "../error.js";
import type { NetworkContext } from "../network.js";
import type { Discoverer, DiscovererReport, DiscoveryReport } from "./discovery.js";
import { failureMessage } from "./discovery.js";
import { createRouter, type Router } from "./index.js";

export type AutoRouterConfig = {
  readonly tokens: readonly EvmAddress[];
  readonly discoverers: readonly Discoverer[];
  readonly context: NetworkContext;
};

export type AutoRouterResult = {
  readonly router: Router;
  readonly report: DiscoveryReport;
};

export const createAutoRouter = async (config: AutoRouterConfig): Promise<AutoRouterResult> => {
  if (config.discoverers.length === 0) {
    throw new EthPricesError("INVALID_CONFIGURATION", "At least one discoverer is required");
  }

  const assets = new Map<string, EvmAddress>();

  for (const token of config.tokens) {
    const canonical = canonicalizeAddress(token);

    assets.set(canonical.toLowerCase(), canonical);
  }

  const router = createRouter();
  const reports: DiscovererReport[] = [];
  let discoveredQuoters = 0;

  for (const discoverer of config.discoverers) {
    const startedAt = performance.now();

    try {
      const result = await discoverer.discover({ tokens: [...assets.values()], context: config.context });
      const discoveredAssets = (result.discoveredAssets ?? []).map(canonicalizeAddress);

      router.addQuoters(result.quoters);
      discoveredQuoters += result.quoters.length;

      for (const asset of discoveredAssets) {
        assets.set(asset.toLowerCase(), asset);
      }

      reports.push({
        identity: discoverer.identity,
        durationMs: performance.now() - startedAt,
        attempted: result.attempted,
        discovered: result.quoters.length,
        skipped: result.skipped,
        failures: result.failures,
      });
    }
    catch (error: unknown) {
      reports.push({
        identity: discoverer.identity,
        durationMs: performance.now() - startedAt,
        attempted: 0,
        discovered: 0,
        skipped: 0,
        failures: [{ target: discoverer.identity, message: failureMessage(error), cause: error }],
      });
    }
  }

  return {
    router,
    report: {
      tokens: config.tokens.map(canonicalizeAddress),
      discoveredAssets: [...assets.values()],
      discoveredQuoters,
      discoverers: reports,
    },
  };
};
