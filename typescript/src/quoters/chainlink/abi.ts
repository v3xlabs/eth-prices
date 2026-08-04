import { type AbiFunction, from } from "ox/AbiFunction";

export const decimals: AbiFunction = from({
  name: "decimals",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "uint8" }],
  stateMutability: "view",
});

export const latestRoundData: AbiFunction = from({
  name: "latestRoundData",
  type: "function",
  inputs: [],
  outputs: [
    { name: "roundId", type: "uint80" },
    { name: "answer", type: "int256" },
    { name: "startedAt", type: "uint256" },
    { name: "updatedAt", type: "uint256" },
    { name: "answeredInRound", type: "uint80" },
  ],
  stateMutability: "view",
});
