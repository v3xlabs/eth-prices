import { type AbiFunction, from } from "ox/AbiFunction";

export const getPool: AbiFunction = from({
  name: "getPool",
  type: "function",
  inputs: [
    { name: "", type: "address" },
    { name: "", type: "address" },
    { name: "", type: "uint24" },
  ],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});
