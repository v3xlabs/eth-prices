import { canonicalizeAddress } from "../../asset.js";
import { EthPricesError } from "../../error.js";
import type { QuoteParams, Quoter } from "../../quoter.js";
import { contractCall } from "../../utils/contract.js";
import * as abi from "./abi.js";

export type ERC4626QuoterParams = {
  networkId: number;
  vaultAddress: `0x${string}`;
  assetAddress: `0x${string}`;
  confidence?: number;
};

export const erc4626Quoter = (params: ERC4626QuoterParams): Quoter => ({
  identity: `erc4626:${canonicalizeAddress(params.vaultAddress)}`,
  assets: [canonicalizeAddress(params.vaultAddress), canonicalizeAddress(params.assetAddress)],
  confidence: params.confidence ?? 0,
  quote: async ({ amountIn, direction, context }: QuoteParams) => {
    if (amountIn < 0n) throw new EthPricesError("INVALID_INPUT", "amountIn must not be negative");

    const fn = direction === "forward" ? abi.convertToAssets : abi.convertToShares;
    const result = await contractCall(context.getProvider(params.networkId), params.vaultAddress, fn, [amountIn], context.blockNumber);

    if (typeof result !== "bigint") throw new EthPricesError("CONTRACT_ERROR", "ERC-4626 returned a malformed amount");

    return result;
  },
});
