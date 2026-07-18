import { type AbiFunction, from } from "ox/AbiFunction";

export const getReserves: AbiFunction = from({
  name: "getReserves",
  type: "function",
  inputs: [],
  outputs: [
    { name: "reserve0", type: "uint112" },
    { name: "reserve1", type: "uint112" },
    { name: "blockTimestampLast", type: "uint32" },
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

export const factoryGetPair: AbiFunction = from({
  name: "getPair",
  type: "function",
  inputs: [
    { name: "", type: "address" },
    { name: "", type: "address" },
  ],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});
