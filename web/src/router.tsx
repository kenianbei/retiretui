import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
  retainSearchParams,
} from "@tanstack/react-router";

import { ComparePage } from "@/compare/page";
import { compareSearch, withSearch } from "@/compare/search";
import { DOMAINS, TOOLS, pageOf } from "@/nav";
import { LedgerPage } from "@/ledger/page";
import { Overview } from "@/overview/overview";
import { NewPlanPage } from "@/onboarding/page";
import { newPlanSearch } from "@/onboarding/steps";
import { DomainPage } from "@/plan/page";
import { planSearch } from "@/plan/search";
import { NotFound } from "@/pages/not-found";
import { Shell } from "@/shell/shell";
import { ToolPage } from "@/tools/page";
import { toolSearch } from "@/tools/search";
import { ledgerSearch, yearSearch, type KeptKey } from "@/year/search";

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
  component: Overview,
});

const ledger = createRoute({
  getParentRoute: () => root,
  path: "/ledger",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: ledgerSearch,
  component: LedgerPage,
});

const compare = createRoute({
  getParentRoute: () => root,
  path: "/compare",
  staticData: { keeps: ["year", "basis", "held"] },
  validateSearch: compareSearch,
  component: ComparePage,
});

const tools = createRoute({
  getParentRoute: () => root,
  path: "/tools/$page",
  staticData: { keeps: ["basis", "held"] },
  validateSearch: toolSearch,
  beforeLoad: ({ params }) => ({ page: pageOf(TOOLS, params.page) }),
  component: ToolPage,
});

const plan = createRoute({
  getParentRoute: () => root,
  path: "/plan/$page",
  validateSearch: planSearch,
  beforeLoad: ({ params }) => ({ page: pageOf(DOMAINS, params.page) }),
  component: DomainPage,
});

const newPlan = createRoute({
  getParentRoute: () => root,
  path: "/new/$step",
  staticData: { isWithoutDocument: true },
  validateSearch: newPlanSearch,
  component: NewPlanPage,
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
