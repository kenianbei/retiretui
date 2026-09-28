import { keepPreviousData, useQuery } from "@tanstack/react-query";

import type { Answer, Replies, Search } from "@/worker";

type Kind = Search["kind"];

/** Each kind's worker, its module loaded before the search it will run. */
const warm = new Map<Kind, Worker>();

/** Each kind's search under way, stopped by what this holds. */
const running = new Map<Kind, () => void>();

function workerFor(kind: Kind): Worker {
  const kept = warm.get(kind);
  if (kept) return kept;
  const worker = new Worker(new URL("./worker.ts", import.meta.url), {
    type: "module",
  });
  warm.set(kind, worker);
  return worker;
}

/**
 * Runs `search` in its kind's worker. A search cannot be interrupted but by
 * terminating its worker, so `signal`, or a newer search of the same kind,
 * does that and leaves a fresh worker warm in its place.
 */
function runSearch<K extends Kind>(
  search: Search & { kind: K },
  signal: AbortSignal,
): Promise<Replies[K]> {
  running.get(search.kind)?.();
  return new Promise((resolve, reject) => {
    const worker = workerFor(search.kind);
    const settle = () => {
      signal.removeEventListener("abort", stop);
      worker.removeEventListener("message", heard);
      worker.removeEventListener("error", failed);
      running.delete(search.kind);
    };
    const stop = () => {
      settle();
      worker.terminate();
      warm.delete(search.kind);
      workerFor(search.kind);
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
      warm.delete(search.kind);
      reject(new Error(event.message));
    };
    signal.addEventListener("abort", stop, { once: true });
    worker.addEventListener("message", heard);
    worker.addEventListener("error", failed);
    running.set(search.kind, stop);
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
 * Every bracket's ladder in `plan` under `constraints`, searched once they
 * name a destination; the last found stays in view while the next is.
 */
export function useLadders(
  plan: string,
  constraints: string,
  isAimed: boolean,
) {
  return useQuery({
    queryKey: ["ladders", plan, constraints],
    queryFn: ({ signal }) =>
      runSearch({ kind: "ladders", plan, constraints }, signal),
    enabled: isAimed,
    placeholderData: keepPreviousData,
    staleTime: Infinity,
    retry: false,
  });
}
