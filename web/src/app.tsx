import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { router } from "@/router";
import { SessionProvider } from "@/session";

/** Draws the app into `container`, once the bindings have loaded. */
export function mount(container: HTMLElement) {
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
