export type EthPricesErrorCode
  = | "ASSET_NOT_FOUND"
    | "AUTO_ROUTER_NO_QUOTERS"
    | "CONTRACT_ERROR"
    | "INVALID_ADDRESS"
    | "INVALID_CONFIGURATION"
    | "INVALID_INPUT"
    | "INVALID_NETWORK"
    | "NO_ROUTE_FOUND"
    | "RATE_UNAVAILABLE";

export class EthPricesError extends Error {
  readonly code: EthPricesErrorCode;
  override readonly cause?: unknown;

  constructor(code: EthPricesErrorCode, message: string, options?: { cause?: unknown; }) {
    super(message);
    this.name = "EthPricesError";
    this.code = code;
    this.cause = options?.cause;
  }
}
