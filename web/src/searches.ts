import { useQuery } from "@tanstack/react-query";
import type { MonteCarloReply } from "@wasm/retiretui_wasm.js";

import type { Answer } from "@/worker";

/**
 * Runs `plan` through random markets in a Worker of its own, which `signal`
 * stops by terminating: a search cannot be interrupted any other way.
 */
function runMarkets(
  plan: string,
  signal: AbortSignal,
): Promise<MonteCarloReply> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.ts", import.meta.url), {
      type: "module",
    });
    const stop = () => {
      worker.terminate();
      reject(new Error("the markets were stopped"));
    };
    signal.addEventListener("abort", stop, { once: true });
    const settle = () => {
      signal.removeEventListener("abort", stop);
      worker.terminate();
    };
    worker.addEventListener("message", (event: MessageEvent<Answer>) => {
      settle();
      const answer = event.data;
      if ("error" in answer) reject(new Error(answer.error));
      else resolve(answer.reply);
    });
    worker.addEventListener("error", (event) => {
      settle();
      reject(new Error(event.message));
    });
    worker.postMessage(plan);
  });
}

/** `plan` through random markets, kept while the page runs: its text is the whole input. */
export function useMonteCarlo(plan: string) {
  return useQuery({
    queryKey: ["monteCarlo", plan],
    queryFn: ({ signal }) => runMarkets(plan, signal),
    staleTime: Infinity,
    retry: false,
  });
}
