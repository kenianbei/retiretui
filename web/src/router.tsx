import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";

import { DOMAINS, TOOLS, pageOf } from "@/nav";
import { GroupedPage, NotFound, Placeholder } from "@/pages/placeholder";
import { Shell } from "@/shell/shell";

const root = createRootRoute({ component: Shell, notFoundComponent: NotFound });

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
  component: () => (
    <Placeholder
      title="Overview"
      holds="Whether the money lasts, what needs attention, and what to do this year."
    />
  ),
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
  beforeLoad: ({ params }) => pageOf(TOOLS, params.page),
  component: () => <GroupedPage pages={TOOLS} />,
});

const plan = createRoute({
  getParentRoute: () => root,
  path: "/plan/$page",
  beforeLoad: ({ params }) => pageOf(DOMAINS, params.page),
  component: () => <GroupedPage pages={DOMAINS} />,
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
