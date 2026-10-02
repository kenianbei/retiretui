import init from "@wasm/retiretui_wasm.js";

import "./index.css";

if (import.meta.env.PROD && "serviceWorker" in navigator) {
  // A registration cut short, by a reload or offline, is tried again on the next load.
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("./sw.js").catch(() => undefined);
  });
}

await init();
const { mount } = await import("@/app");

const container = document.getElementById("root");
if (!container) throw new Error("index.html has no #root");
mount(container);
window.startup.started();
