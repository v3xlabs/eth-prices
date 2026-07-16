import { type AbiFunction, from } from "ox/AbiFunction";

export const slot0: AbiFunction = from({
  name: "slot0",
  type: "function",
  inputs: [],
  outputs: [
    { name: "sqrtPriceX96", type: "uint160" },
    { name: "tick", type: "int24" },
    { name: "observationIndex", type: "uint16" },
    { name: "observationCardinality", type: "uint16" },
    { name: "observationCardinalityNext", type: "uint16" },
    { name: "feeProtocol", type: "uint8" },
    { name: "unlocked", type: "bool" },
  ],
  stateMutability: "view",
});

export const token0: AbiFunction = from({
  name: "token0",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});

export const token1: AbiFunction = from({
  name: "token1",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});

export const liquidity: AbiFunction = from({
  name: "liquidity",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "uint128" }],
  stateMutability: "view",
});
