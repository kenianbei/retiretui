import { useQuery } from "@tanstack/react-query";

import type {
  SearchAnswer,
  SearchName,
  SearchReply,
  SearchRequest,
} from "@/worker";

/**
 * Runs `search` over `plan` in a Worker of its own, which `signal` stops by
 * terminating: a search cannot be interrupted any other way.
 */
function run<Name extends SearchName>(
  search: Name,
  plan: string,
  signal: AbortSignal,
): Promise<SearchReply<Name>> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.ts", import.meta.url), {
      type: "module",
    });
    const stop = () => {
      worker.terminate();
      reject(new Error(`the ${search} search was stopped`));
    };
    signal.addEventListener("abort", stop, { once: true });
    const settle = () => {
      signal.removeEventListener("abort", stop);
      worker.terminate();
    };
    worker.addEventListener("message", (event: MessageEvent<SearchAnswer>) => {
      settle();
      const answer = event.data;
      if ("error" in answer) reject(new Error(answer.error));
      else resolve(answer.reply as SearchReply<Name>);
    });
    worker.addEventListener("error", (event) => {
      settle();
      reject(new Error(event.message));
    });
    worker.postMessage({ search, plan } satisfies SearchRequest);
  });
}

/** `search` over `plan`, kept for as long as the page runs: a plan's text is its whole input. */
export function useSearch<Name extends SearchName>(search: Name, plan: string) {
  return useQuery({
    queryKey: [search, plan],
    queryFn: ({ signal }) => run(search, plan, signal),
    staleTime: Infinity,
    retry: false,
  });
}
