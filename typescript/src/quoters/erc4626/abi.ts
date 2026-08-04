import { type AbiFunction, from } from "ox/AbiFunction";

export const asset: AbiFunction = from({
  name: "asset",
  type: "function",
  inputs: [],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});

export const convertToAssets: AbiFunction = from({
  name: "convertToAssets",
  type: "function",
  inputs: [{ name: "shares", type: "uint256" }],
  outputs: [{ name: "", type: "uint256" }],
  stateMutability: "view",
});

export const convertToShares: AbiFunction = from({
  name: "convertToShares",
  type: "function",
  inputs: [{ name: "assets", type: "uint256" }],
  outputs: [{ name: "", type: "uint256" }],
  stateMutability: "view",
});
