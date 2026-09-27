import { RouterProvider } from "@tanstack/react-router";
import init from "@wasm/retiretui_wasm.js";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { router } from "@/router";
import { SessionProvider } from "@/session";

import "./index.css";

const container = document.getElementById("root");
if (!container) throw new Error("index.html has no #root");

await init();

createRoot(container).render(
  <StrictMode>
    <SessionProvider>
      <RouterProvider router={router} />
    </SessionProvider>
  </StrictMode>,
);
