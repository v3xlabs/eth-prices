import { canonicalizeAddress, type EvmAddress } from "../../asset.js";
import type { Quoter } from "../../quoter.js";
import type { Discoverer, DiscoveryFailure } from "../../router/discovery.js";
import { failureMessage } from "../../router/discovery.js";
import { settleMap } from "../../utils/concurrency.js";
import { contractCall } from "../../utils/contract.js";
import * as abi from "./abi.js";
import { erc4626Quoter } from "./index.js";

export type ERC4626DiscovererOptions = {
  readonly networkId: number;
  readonly concurrency?: number;
};

export const erc4626Discoverer = (options: ERC4626DiscovererOptions): Discoverer => ({
  identity: `erc4626:${options.networkId}`,
  discover: async ({ tokens, context }) => {
    const provider = context.getProvider(options.networkId);
    const results = await settleMap(tokens, options.concurrency ?? 16, async (vaultAddress) => {
      const result = await contractCall(provider, vaultAddress, abi.asset, [], context.blockNumber);

      if (!isAddressResult(result)) throw new TypeError("ERC-4626 asset() returned malformed data");

      const underlyingAddress = canonicalizeAddress(result);

      return {
        underlyingAddress,
        quoter: erc4626Quoter({
          networkId: options.networkId,
          vaultAddress,
          tokenAddress: underlyingAddress,
          confidence: 50,
        }),
      };
    });
    const quoters: Quoter[] = [];
    const discoveredAssets: EvmAddress[] = [];
    const failures: DiscoveryFailure[] = [];

    for (const { input: token, result } of results) {
      if (result.status === "fulfilled") {
        quoters.push(result.value.quoter);
        discoveredAssets.push(result.value.underlyingAddress);
      }
      else {
        failures.push({ target: token, message: failureMessage(result.reason), cause: result.reason });
      }
    }

    return { quoters, discoveredAssets, attempted: tokens.length, skipped: failures.length, failures };
  },
});

const isAddressResult = (value: unknown): value is EvmAddress => typeof value === "string" && value.startsWith("0x");
