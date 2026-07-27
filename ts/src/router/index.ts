import { canonicalizeAsset } from "../asset.js";
import { EthPricesError } from "../error.js";
import type { QuoteParams as QuoteParameters, Quoter, RouteStep } from "../quoter.js";
import { quoteRoute, type Route } from "./route.js";

const MAX_CONFIDENCE = 100;

type Edge = {
  readonly from: string;
  readonly to: string;
  readonly quoter: Quoter;
  readonly direction: "forward" | "reverse";
};

export type Router = {
  addQuoter(quoter: Quoter): void;
  addQuoters(quoters: Iterable<Quoter>): void;
  compute(inputAsset: string, outputAsset: string): Route;
  quote(inputAsset: string, outputAsset: string, parameters: Omit<QuoteParameters, "direction">): Promise<bigint>;
  quoters(): readonly Quoter[];
};

const edgeCost = (quoter: Quoter): number => MAX_CONFIDENCE + 1 - Math.min(quoter.confidence, MAX_CONFIDENCE);

export const createRouter = (initialQuoters: Iterable<Quoter> = []): Router => {
  const adjacency = new Map<string, Edge[]>();
  const orderedQuoters: Quoter[] = [];

  const validateQuoter = (quoter: Quoter): void => {
    if (!Number.isSafeInteger(quoter.confidence) || quoter.confidence < 0) {
      throw new EthPricesError("INVALID_CONFIGURATION", `Invalid confidence for ${quoter.identity}`);
    }

    const from = canonicalizeAsset(quoter.assets[0]);
    const to = canonicalizeAsset(quoter.assets[1]);

    if (from === to) {
      throw new EthPricesError("INVALID_CONFIGURATION", `Quoter ${quoter.identity} connects an asset to itself`);
    }
  };

  const insertQuoter = (quoter: Quoter): void => {
    const from = canonicalizeAsset(quoter.assets[0]);
    const to = canonicalizeAsset(quoter.assets[1]);

    const forward: Edge = { from, to, quoter, direction: "forward" };
    const reverse: Edge = { from: to, to: from, quoter, direction: "reverse" };

    adjacency.set(from, [...(adjacency.get(from) ?? []), forward]);
    adjacency.set(to, [...(adjacency.get(to) ?? []), reverse]);
    orderedQuoters.push(quoter);
  };

  const addQuoter = (quoter: Quoter): void => {
    validateQuoter(quoter);
    insertQuoter(quoter);
  };

  const addQuoters = (quoters: Iterable<Quoter>): void => {
    const candidates = [...quoters];

    for (const quoter of candidates) validateQuoter(quoter);

    for (const quoter of candidates) insertQuoter(quoter);
  };

  const compute = (inputAsset: string, outputAsset: string): Route => {
    const start = canonicalizeAsset(inputAsset);
    const goal = canonicalizeAsset(outputAsset);

    if (!adjacency.has(start)) throw new EthPricesError("ASSET_NOT_FOUND", `Asset not found: ${start}`);

    if (!adjacency.has(goal)) throw new EthPricesError("ASSET_NOT_FOUND", `Asset not found: ${goal}`);

    if (start === goal) return { path: [], inputAsset: start, outputAsset: goal };

    const distances = new Map<string, number>([[start, 0]]);
    const previous = new Map<string, Edge>();
    const pending = new Set<string>(adjacency.keys());

    while (pending.size > 0) {
      let current: string | undefined;
      let currentDistance = Infinity;

      for (const candidate of pending) {
        const distance = distances.get(candidate) ?? Infinity;

        if (distance < currentDistance) {
          current = candidate;
          currentDistance = distance;
        }
      }

      if (current === undefined || currentDistance === Infinity) break;

      pending.delete(current);

      if (current === goal) break;

      for (const edge of adjacency.get(current) ?? []) {
        if (!pending.has(edge.to)) continue;

        const candidateDistance = currentDistance + edgeCost(edge.quoter);
        const knownDistance = distances.get(edge.to) ?? Infinity;

        if (candidateDistance < knownDistance) {
          distances.set(edge.to, candidateDistance);
          previous.set(edge.to, edge);
        }
      }
    }

    if (!previous.has(goal)) {
      throw new EthPricesError("NO_ROUTE_FOUND", `No route found from ${start} to ${goal}`);
    }

    const reversedPath: RouteStep[] = [];
    let cursor = goal;

    while (cursor !== start) {
      const edge = previous.get(cursor);

      if (edge === undefined) {
        throw new EthPricesError("NO_ROUTE_FOUND", `No route found from ${start} to ${goal}`);
      }

      reversedPath.push({ quoter: edge.quoter, direction: edge.direction });
      cursor = edge.from;
    }

    return { path: reversedPath.reverse(), inputAsset: start, outputAsset: goal };
  };

  const quote = async (
    inputAsset: string,
    outputAsset: string,
    parameters: Omit<QuoteParameters, "direction">,
  ): Promise<bigint> => quoteRoute(compute(inputAsset, outputAsset), parameters);

  const quoters = (): readonly Quoter[] => [...orderedQuoters];

  addQuoters(initialQuoters);

  return { addQuoter, addQuoters, compute, quote, quoters };
};
