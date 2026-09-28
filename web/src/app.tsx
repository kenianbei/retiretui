import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { preloadPages, router } from "@/router";
import { SessionProvider } from "@/session";

/** Once the page is idle, loads what it has not shown yet. */
function whenIdle(run: () => void) {
  if ("requestIdleCallback" in window) requestIdleCallback(run);
  else setTimeout(run, IDLE_MS);
}

/** How long a browser without idle callbacks waits before loading the rest. */
const IDLE_MS = 2000;

/** Draws the app into `container`, once the bindings have loaded. */
export function mount(container: HTMLElement) {
  whenIdle(preloadPages);
  createRoot(container).render(
    <StrictMode>
      <QueryClientProvider client={new QueryClient()}>
        <SessionProvider>
          <RouterProvider router={router} />
        </SessionProvider>
      </QueryClientProvider>
    </StrictMode>,
  );
}
