import type { EvmAddress } from "../asset.js";
import type { NetworkContext } from "../network.js";
import type { Quoter } from "../quoter.js";

export type DiscoveryFailure = {
  readonly target: string;
  readonly message: string;
  readonly cause?: unknown;
};

export type DiscovererResult = {
  readonly quoters: readonly Quoter[];
  readonly discoveredAssets?: readonly EvmAddress[];
  readonly attempted: number;
  readonly skipped: number;
  readonly failures: readonly DiscoveryFailure[];
};

export type DiscoveryInput = {
  readonly tokens: readonly EvmAddress[];
  readonly context: NetworkContext;
};

export type Discoverer = {
  readonly identity: string;
  discover(input: DiscoveryInput): Promise<DiscovererResult>;
};

export type DiscovererReport = {
  readonly identity: string;
  readonly durationMs: number;
  readonly attempted: number;
  readonly discovered: number;
  readonly skipped: number;
  readonly failures: readonly DiscoveryFailure[];
};

export type DiscoveryReport = {
  readonly tokens: readonly EvmAddress[];
  readonly discoveredAssets: readonly EvmAddress[];
  readonly discoveredQuoters: number;
  readonly discoverers: readonly DiscovererReport[];
};

export const failureMessage = (error: unknown): string => (error instanceof Error ? error.message : String(error));
