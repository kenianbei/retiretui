import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  redirect,
  retainSearchParams,
} from "@tanstack/react-router";

import { compareSearch, withSearch } from "@/compare/search";
import { DOMAINS, TOOLS, pageOf } from "@/nav";
import { newPlanSearch } from "@/onboarding/steps";
import { planSearch } from "@/plan/search";
import { NotFound } from "@/pages/not-found";
import { Shell } from "@/shell/shell";
import { toolSearch } from "@/tools/search";
import { ledgerSearch, yearSearch, type KeptKey } from "@/year/search";

/**
 * The pages, each loaded the first time it is shown, so that what charts
 * and edits stays out of the first load.
 */
const PAGES = {
  overview: lazyRouteComponent(() => import("@/overview/overview"), "Overview"),
  ledger: lazyRouteComponent(() => import("@/ledger/page"), "LedgerPage"),
  compare: lazyRouteComponent(() => import("@/compare/page"), "ComparePage"),
  tools: lazyRouteComponent(() => import("@/tools/page"), "ToolPage"),
  plan: lazyRouteComponent(() => import("@/plan/page"), "DomainPage"),
  newPlan: lazyRouteComponent(() => import("@/onboarding/page"), "NewPlanPage"),
};

/**
 * Loads every page not yet shown, so that each is at hand offline; one
 * whose load is cut short loads when it is shown instead.
 */
export function preloadPages() {
  for (const page of Object.values(PAGES)) {
    page.preload?.()?.catch(() => undefined);
  }
}

const root = createRootRoute({
  component: Shell,
  notFoundComponent: NotFound,
  validateSearch: withSearch,
  search: { middlewares: [retainSearchParams(["with"])] },
});

const index = createRoute({
  getParentRoute: () => root,
  path: "/",
  beforeLoad: () => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router's own redirect
    throw redirect({ to: "/overview" });
  },
});

const overview = createRoute({
  getParentRoute: () => root,
  path: "/overview",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: yearSearch,
  component: PAGES.overview,
});

const ledger = createRoute({
  getParentRoute: () => root,
  path: "/ledger",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: ledgerSearch,
  component: PAGES.ledger,
});

const compare = createRoute({
  getParentRoute: () => root,
  path: "/compare",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: compareSearch,
  component: PAGES.compare,
});

const tools = createRoute({
  getParentRoute: () => root,
  path: "/tools/$page",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: toolSearch,
  beforeLoad: ({ params }) => ({ page: pageOf(TOOLS, params.page) }),
  component: PAGES.tools,
});

const plan = createRoute({
  getParentRoute: () => root,
  path: "/plan/$page",
  validateSearch: planSearch,
  beforeLoad: ({ params }) => ({ page: pageOf(DOMAINS, params.page) }),
  component: PAGES.plan,
});

const newPlan = createRoute({
  getParentRoute: () => root,
  path: "/new/$step",
  staticData: { isWithoutDocument: true },
  validateSearch: newPlanSearch,
  component: PAGES.newPlan,
});

export const router = createRouter({
  routeTree: root.addChildren([
    index,
    overview,
    ledger,
    compare,
    tools,
    plan,
    newPlan,
  ]),
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
  interface StaticDataRouteOption {
    /** A page shown whether or not a document is open. */
    isWithoutDocument?: boolean;
    /** What of the address a tab's link to the route carries over. */
    keeps?: readonly KeptKey[];
  }
}
