import { keepPreviousData, useQuery } from "@tanstack/react-query";

import type { Answer, Replies, Search } from "@/worker";

type Kind = Search["kind"];

/**
 * Which worker a search runs in: its kind, and for a ladder the account it
 * fills, so that two owners' ladders run side by side rather than one
 * stopping the other.
 */
function laneOf(search: Search): string {
  return search.kind === "ladders" ? `ladders:${search.lane}` : search.kind;
}

/** Each lane's worker, its module loaded before the search it will run. */
const warm = new Map<string, Worker>();

/** Each lane's search under way, stopped by what this holds. */
const running = new Map<string, () => void>();

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
 * Runs `search` in its lane's worker. A search cannot be interrupted but by
 * terminating its worker, so `signal`, or a newer search in the same lane,
 * does that and leaves a fresh worker warm in its place.
 */
function runSearch<K extends Kind>(
  search: Search & { kind: K },
  signal: AbortSignal,
): Promise<Replies[K]> {
  const lane = laneOf(search);
  running.get(lane)?.();
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
      workerFor(lane);
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

/** `plan` through random markets, kept while the page runs: its text is the whole input. */
export function useMonteCarlo(plan: string) {
  return useQuery({
    queryKey: ["monteCarlo", plan],
    queryFn: ({ signal }) => runSearch({ kind: "monteCarlo", plan }, signal),
    staleTime: Infinity,
    retry: false,
  });
}

/**
 * Every bracket's ladder into `destination` in `plan` under `constraints`,
 * which aim there, searched only where `isSearchable`; the last found stays
 * in view while the next is.
 */
export function useLadders(
  plan: string,
  constraints: string,
  destination: string,
  isSearchable: boolean,
) {
  return useQuery({
    queryKey: ["ladders", plan, constraints],
    queryFn: ({ signal }) =>
      runSearch(
        { kind: "ladders", plan, constraints, lane: destination },
        signal,
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
