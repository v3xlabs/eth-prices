import { type AbiFunction, from } from "ox/AbiFunction";

export const findPoolsForCoins: AbiFunction = from({
  name: "find_pools_for_coins",
  type: "function",
  inputs: [
    { name: "_from", type: "address" },
    { name: "_to", type: "address" },
  ],
  outputs: [{ name: "", type: "address[]" }],
  stateMutability: "view",
});

export const getCoinIndices: AbiFunction = from({
  name: "get_coin_indices",
  type: "function",
  inputs: [
    { name: "_pool", type: "address" },
    { name: "_from", type: "address" },
    { name: "_to", type: "address" },
  ],
  outputs: [
    { name: "i", type: "int128" },
    { name: "j", type: "int128" },
    { name: "is_underlying", type: "bool" },
  ],
  stateMutability: "view",
});

export const getBalances: AbiFunction = from({
  name: "get_balances",
  type: "function",
  inputs: [{ name: "_pool", type: "address" }],
  outputs: [{ name: "", type: "uint256[8]" }],
  stateMutability: "view",
});

export const getDyStableSwap: AbiFunction = from({
  name: "get_dy",
  type: "function",
  inputs: [
    { name: "i", type: "int128" },
    { name: "j", type: "int128" },
    { name: "dx", type: "uint256" },
  ],
  outputs: [{ name: "", type: "uint256" }],
  stateMutability: "view",
});

export const getDyCrypto: AbiFunction = from({
  name: "get_dy",
  type: "function",
  inputs: [
    { name: "i", type: "uint256" },
    { name: "j", type: "uint256" },
    { name: "dx", type: "uint256" },
  ],
  outputs: [{ name: "", type: "uint256" }],
  stateMutability: "view",
});
