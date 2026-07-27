import { canonicalizeAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParameters, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import * as abi from "./abi.js";

export type ERC4626QuoterParams = {
  networkId: number;
  vaultAddress: `0x${string}`;
  assetAddress: `0x${string}`;
  confidence?: number;
};

export const erc4626Quoter = (parameters: ERC4626QuoterParams): Quoter => ({
  identity: `erc4626:${canonicalizeAddress(parameters.vaultAddress)}`,
  assets: [canonicalizeAddress(parameters.vaultAddress), canonicalizeAddress(parameters.assetAddress)],
  confidence: parameters.confidence ?? 0,
  quote: async ({ amountIn, direction, context }: QuoteParameters) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    const function_ = direction === "forward" ? abi.convertToAssets : abi.convertToShares;
    const result = await contractCall(context.getProvider(parameters.networkId), parameters.vaultAddress, function_, [amountIn], context.blockNumber);

    if (typeof result !== "bigint") throw new EthPricesError("CONTRACT_ERROR", "ERC-4626 returned a malformed amount");

    return result;
  },
});
