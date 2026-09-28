import init, {
  claims,
  ladders,
  monteCarlo,
  type ClaimsOptions,
  type LaddersReply,
  type MonteCarloReply,
} from "@wasm/retiretui_wasm.js";

/** What a search is asked, by the kind of search it is. */
export type Search =
  | { kind: "monteCarlo"; plan: string }
  | { kind: "ladders"; plan: string; constraints: string; destination: string }
  | { kind: "claims"; plan: string; held: string[] };

/** What each kind of search replies. */
export interface Replies {
  monteCarlo: MonteCarloReply;
  ladders: LaddersReply;
  claims: ClaimsOptions;
}

/** A search's reply, off the page's thread, or why there is none. */
export type Answer<Kind extends Search["kind"] = Search["kind"]> =
  | { reply: Replies[Kind] }
  | { error: string };

declare const self: DedicatedWorkerGlobalScope;

const ready = init();

function answer(search: Search): Replies[Search["kind"]] {
  switch (search.kind) {
    case "monteCarlo":
      return monteCarlo(search.plan);
    case "ladders":
      return ladders(search.plan, search.constraints, search.destination);
    case "claims":
      return claims(search.plan, search.held);
  }
}

self.addEventListener("message", (event: MessageEvent<Search>) => {
  void ready.then(() => {
    try {
      self.postMessage({ reply: answer(event.data) } satisfies Answer);
    } catch (thrown) {
      const error = thrown instanceof Error ? thrown.message : String(thrown);
      self.postMessage({ error } satisfies Answer);
    }
  });
});
