import init from "@wasm/retiretui_wasm.js";

import "./index.css";

await init();
const { mount } = await import("@/app");

const container = document.getElementById("root");
if (!container) throw new Error("index.html has no #root");
mount(container);
