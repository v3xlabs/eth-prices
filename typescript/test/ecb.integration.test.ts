import { describe, expect, it } from "vitest";

import { createNetworkContext } from "../src/network.js";
import { createEcbRateSource, ecbQuoter } from "../src/quoters/ecb/index.js";

describe("ECB live integration", () => {
  it("quotes a fixed historical date", async () => {
    const source = createEcbRateSource();
    const quoter = ecbQuoter({ quoteSymbol: "usd", rateSource: source });
    const context = createNetworkContext({}, { fiatTimestamp: 1_704_067_200n });
    const amount = await quoter.quote({ amountIn: 1_000_000n, direction: "forward", context });

    expect(amount).toBeGreaterThan(1_000_000n);
    expect(amount).toBeLessThan(2_000_000n);
  }, 30_000);

  it("requires fiat time", async () => {
    const source = createEcbRateSource();
    const quoter = ecbQuoter({ quoteSymbol: "usd", rateSource: source });

    await expect(quoter.quote({
      amountIn: 1_000_000n,
      direction: "forward",
      context: createNetworkContext({}),
    })).rejects.toThrow("fiat timestamp");
  });
});
