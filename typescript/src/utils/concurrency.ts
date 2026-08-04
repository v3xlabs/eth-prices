import { EthPricesError } from "../error.js";

export type SettledItem<Input, Output> = {
  readonly input: Input;
  readonly result: PromiseSettledResult<Output>;
};

export const settleMap = async <Input, Output>(
  inputs: readonly Input[],
  concurrency: number,
  operation: (input: Input) => Promise<Output>,
): Promise<Array<SettledItem<Input, Output>>> => {
  if (!Number.isSafeInteger(concurrency) || concurrency <= 0) {
    throw new EthPricesError("INVALID_CONFIGURATION", "concurrency must be a positive safe integer");
  }

  const work = inputs.map((input, index) => ({ input, index }));
  const results: Array<SettledItem<Input, Output> | undefined> = Array.from({ length: inputs.length });
  let cursor = 0;

  const worker = async (): Promise<void> => {
    while (cursor < work.length) {
      const item = work[cursor];

      cursor += 1;

      if (item === undefined) return;

      try {
        const value = await operation(item.input);

        results[item.index] = { input: item.input, result: { status: "fulfilled", value } };
      }
      catch (error: unknown) {
        results[item.index] = { input: item.input, result: { status: "rejected", reason: error } };
      }
    }
  };

  const workerCount = Math.min(concurrency, inputs.length);

  await Promise.all(Array.from({ length: workerCount }, worker));

  return results.flatMap(result => (result === undefined ? [] : [result]));
};
