import init, {
  monteCarlo,
  type MonteCarloReply,
} from "@wasm/retiretui_wasm.js";

/** A plan's text through random markets, off the page's thread. */
export type Answer = { reply: MonteCarloReply } | { error: string };

declare const self: DedicatedWorkerGlobalScope;

const ready = init();

self.addEventListener("message", (event: MessageEvent<string>) => {
  void ready.then(() => {
    try {
      self.postMessage({ reply: monteCarlo(event.data) } satisfies Answer);
    } catch (thrown) {
      const error = thrown instanceof Error ? thrown.message : String(thrown);
      self.postMessage({ error } satisfies Answer);
    }
  });
});
