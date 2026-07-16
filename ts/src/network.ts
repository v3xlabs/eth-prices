import { EthPricesError } from "./error.js";
import type { RpcProvider } from "./utils/contract.js";

export type NetworkContextOptions = {
  blockNumber?: bigint;
  fiatTimestamp?: bigint;
};

export type NetworkContext = {
  readonly blockNumber: bigint | undefined;
  readonly fiatTimestamp: bigint | undefined;
  getProvider(networkId: number): RpcProvider;
};

export const createNetworkContext = (
  providers: Readonly<Record<number, RpcProvider>>,
  options: NetworkContextOptions = {},
): NetworkContext => {
  if (options.blockNumber !== undefined && options.blockNumber < 0n) {
    throw new EthPricesError("INVALID_INPUT", "blockNumber must not be negative");
  }

  if (options.fiatTimestamp !== undefined && options.fiatTimestamp < 0n) {
    throw new EthPricesError("INVALID_INPUT", "fiatTimestamp must not be negative");
  }

  return {
    blockNumber: options.blockNumber,
    fiatTimestamp: options.fiatTimestamp,
    getProvider: (networkId: number) => {
      const provider = providers[networkId];

      if (provider === undefined) {
        throw new EthPricesError("INVALID_NETWORK", `No provider configured for network ${networkId}`);
      }

      return provider;
    },
  };
};
