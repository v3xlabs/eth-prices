import { type AbiFunction, from } from "ox/AbiFunction";

export const getPair: AbiFunction = from({
  name: "getPair",
  type: "function",
  inputs: [
    { name: "", type: "address" },
    { name: "", type: "address" },
  ],
  outputs: [{ name: "", type: "address" }],
  stateMutability: "view",
});
