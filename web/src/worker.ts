import init, { monteCarlo } from "@wasm/retiretui_wasm.js";

/** What a Worker runs over a plan's text, off the page's thread. */
export const SEARCHES = { monteCarlo };

export type SearchName = keyof typeof SEARCHES;

export type SearchReply<Name extends SearchName> = ReturnType<
  (typeof SEARCHES)[Name]
>;

export interface SearchRequest {
  search: SearchName;
  plan: string;
}

export type SearchAnswer = { reply: unknown } | { error: string };

declare const self: DedicatedWorkerGlobalScope;

const ready = init();

self.addEventListener("message", (event: MessageEvent<SearchRequest>) => {
  void ready.then(() => {
    const { search, plan } = event.data;
    try {
      self.postMessage({
        reply: SEARCHES[search](plan),
      } satisfies SearchAnswer);
    } catch (thrown) {
      const error = thrown instanceof Error ? thrown.message : String(thrown);
      self.postMessage({ error } satisfies SearchAnswer);
    }
  });
});
