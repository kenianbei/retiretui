import { keepPreviousData, useQueries, useQuery } from "@tanstack/react-query";
import type { Searched } from "@wasm/retiretui_wasm.js";

import type { Answer, Replies, Search } from "@/worker";

type Kind = Search["kind"];

/** Each lane's worker, its module loaded before the search it will run. */
const warm = new Map<string, Worker>();

/** Each lane's search under way, stopped by what this holds. */
const running = new Map<string, () => void>();

/** Lanes no longer wanted, which a stopped search leaves without a worker. */
const released = new Set<string>();

function workerFor(lane: string): Worker {
  const kept = warm.get(lane);
  if (kept) return kept;
  const worker = new Worker(new URL("./worker.ts", import.meta.url), {
    type: "module",
  });
  warm.set(lane, worker);
  return worker;
}

/**
 * Runs `search` in the worker of its `lane`, its kind unless searches of
 * the kind run side by side. A search cannot be interrupted but by
 * terminating its worker, so `signal`, or a newer search in the same lane,
 * does that and leaves a fresh worker warm in its place.
 */
function runSearch<K extends Kind>(
  search: Search & { kind: K },
  signal: AbortSignal,
  lane: string = search.kind,
): Promise<Replies[K]> {
  running.get(lane)?.();
  released.delete(lane);
  return new Promise((resolve, reject) => {
    const worker = workerFor(lane);
    const settle = () => {
      signal.removeEventListener("abort", stop);
      worker.removeEventListener("message", heard);
      worker.removeEventListener("error", failed);
      running.delete(lane);
    };
    const stop = () => {
      settle();
      worker.terminate();
      warm.delete(lane);
      if (!released.has(lane)) workerFor(lane);
      reject(new Error("the search was stopped"));
    };
    const heard = (event: MessageEvent<Answer<K>>) => {
      settle();
      const answer = event.data;
      if ("error" in answer) reject(new Error(answer.error));
      else resolve(answer.reply);
    };
    const failed = (event: ErrorEvent) => {
      settle();
      warm.delete(lane);
      reject(new Error(event.message));
    };
    signal.addEventListener("abort", stop, { once: true });
    worker.addEventListener("message", heard);
    worker.addEventListener("error", failed);
    running.set(lane, stop);
    worker.postMessage(search);
  });
}

/** Terminates `lane`'s worker and any search in it, loading no other. */
export function releaseLane(lane: string) {
  released.add(lane);
  running.get(lane)?.();
  warm.get(lane)?.terminate();
  warm.delete(lane);
}

/** A market tool's search: through random markets, or from every historical start. */
export type MarketKind = "monteCarlo" | "historical";

/**
 * `plan` through `kind`'s markets, searched only where `isSearchable` and
 * kept while the page runs: its text is the whole input. The last found
 * stays in view while the next is.
 */
export function useMarkets(
  kind: MarketKind,
  plan: string,
  isSearchable: boolean,
) {
  return useQuery({
    queryKey: [kind, plan],
    queryFn: ({ signal }) => runSearch({ kind, plan }, signal),
    enabled: isSearchable,
    placeholderData: keepPreviousData,
    staleTime: Infinity,
    retry: false,
  });
}

/** Where a plan's search through random markets has got to. */
function searchedOf(found: {
  data?: { success_rate: number };
  error: Error | null;
  isFetching: boolean;
}): Searched {
  if (found.data) return { kind: "rate", rate: found.data.success_rate };
  if (found.error && !found.isFetching) return { kind: "failed" };
  return { kind: "waiting" };
}

function searchedOfEach(found: Parameters<typeof searchedOf>[0][]): Searched[] {
  return found.map(searchedOf);
}

/**
 * Where each of `plans` has got to through random markets, each searched in
 * its own `lane` so that they run side by side, and only where
 * `isSearchable`; a plan's text keys its search as `useMarkets` does, so
 * the same plan is searched once.
 */
export function useSuccesses(
  plans: readonly { plan: string; lane: string }[],
  isSearchable: boolean,
): Searched[] {
  return useQueries({
    queries: plans.map(({ plan, lane }) => ({
      queryKey: ["monteCarlo", plan],
      queryFn: ({ signal }: { signal: AbortSignal }) =>
        runSearch({ kind: "monteCarlo", plan }, signal, lane),
      enabled: isSearchable && plan !== "",
      staleTime: Infinity,
      retry: false,
    })),
    combine: searchedOfEach,
  });
}

/**
 * Every bracket's ladder into `destination` in `plan` under `constraints`,
 * searched only where `isSearchable`, each destination in a lane of its own
 * so that two owners' ladders run side by side; the last found stays in
 * view while the next is.
 */
export function useLadders(
  plan: string,
  constraints: string,
  destination: string,
  isSearchable: boolean,
) {
  return useQuery({
    queryKey: ["ladders", plan, constraints, destination],
    queryFn: ({ signal }) =>
      runSearch(
        { kind: "ladders", plan, constraints, destination },
        signal,
        `ladders:${destination}`,
      ),
    enabled: isSearchable,
    placeholderData: keepPreviousData,
    staleTime: Infinity,
    retry: false,
  });
}

/**
 * Every claim age in `plan` for the household but the people `held`,
 * searched only where `isSearchable`; the last found stays in view while
 * the next is.
 */
export function useClaims(
  plan: string,
  held: readonly string[],
  isSearchable: boolean,
) {
  return useQuery({
    queryKey: ["claims", plan, held],
    queryFn: ({ signal }) =>
      runSearch({ kind: "claims", plan, held: [...held] }, signal),
    enabled: isSearchable,
    placeholderData: keepPreviousData,
    staleTime: Infinity,
    retry: false,
  });
}
