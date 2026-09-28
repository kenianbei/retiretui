import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";

import { DOMAINS, TOOLS, pageOf } from "@/nav";
import { Overview } from "@/overview/overview";
import type { Basis } from "@/overview/words";
import { planSearch } from "@/plan/search";
import { GroupedPage, NotFound, Placeholder } from "@/pages/placeholder";
import { Shell } from "@/shell/shell";

const root = createRootRoute({ component: Shell, notFoundComponent: NotFound });

const index = createRoute({
  getParentRoute: () => root,
  path: "/",
  beforeLoad: () => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router's own redirect
    throw redirect({ to: "/overview", search: { basis: "today" } });
  },
});

const overview = createRoute({
  getParentRoute: () => root,
  path: "/overview",
  validateSearch: (search: Record<string, unknown>): { basis: Basis } => ({
    basis: search.basis === "nominal" ? "nominal" : "today",
  }),
  component: Overview,
});

const ledger = createRoute({
  getParentRoute: () => root,
  path: "/ledger",
  component: () => (
    <Placeholder
      title="Ledger"
      holds="The plan year by year: every account's flows, income and tax."
    />
  ),
});

const compare = createRoute({
  getParentRoute: () => root,
  path: "/compare",
  component: () => (
    <Placeholder
      title="Compare"
      holds="The plan beside other plans in the workspace."
    />
  ),
});

const tools = createRoute({
  getParentRoute: () => root,
  path: "/tools/$page",
  beforeLoad: ({ params }) => ({ page: pageOf(TOOLS, params.page) }),
  component: GroupedPage,
});

const plan = createRoute({
  getParentRoute: () => root,
  path: "/plan/$page",
  validateSearch: planSearch,
  beforeLoad: ({ params }) => ({ page: pageOf(DOMAINS, params.page) }),
  component: GroupedPage,
});

export const router = createRouter({
  routeTree: root.addChildren([index, overview, ledger, compare, tools, plan]),
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
