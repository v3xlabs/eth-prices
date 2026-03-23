import { describe, expect, it } from "vitest";

import { createQuoter } from "../src/index";

const TOKEN_A = "0x0000000000000000000000000000000000000001";
const TOKEN_B = "0x0000000000000000000000000000000000000002";

describe("wasm quoter bindings", () => {
  it("creates a fixed route and quotes through it", async () => {
    const quoter = await createQuoter({
      rpcUrl: "http://127.0.0.1:8545",
      quoters: {
        fixed: [
          { token_in: TOKEN_A, token_out: TOKEN_B, fixed_rate: 2 },
          { token_in: TOKEN_B, token_out: "fiat:usd", fixed_rate: 3 },
        ],
      },
    });

    const route = quoter.computeRoute(TOKEN_A, "fiat:usd");
    const view = route.toJSON();

    expect(view.inputToken).toBe(TOKEN_A);
    expect(view.outputToken).toBe("fiat:usd");
    expect(view.path).toHaveLength(2);
    expect(view.path[0].direction).toBe("Forward");
    expect(view.path[1].direction).toBe("Forward");

    const amountOut = await quoter.quoteRoute(route, "100", 1n);
    expect(amountOut).toBe("600");
  });

  it("supports convenience quote API", async () => {
    const quoter = await createQuoter({
      rpcUrl: "http://127.0.0.1:8545",
      quoters: {
        fixed: [{ token_in: TOKEN_A, token_out: "fiat:usd", fixed_rate: 1.25 }],
      },
    });

    const amountOut = await quoter.getRate(TOKEN_A, "fiat:usd", "80", 1n);
    expect(amountOut).toBe("100");

    const amountOutViaRequest = await quoter.quote({
      inputToken: TOKEN_A,
      outputToken: "fiat:usd",
      amountIn: "80",
      block: 1n,
    });
    expect(amountOutViaRequest).toBe("100");
  });
});
