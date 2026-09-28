import { notFound } from "@tanstack/react-router";
import { domains } from "@wasm/retiretui_wasm.js";
import {
  ChartNoAxesColumn,
  Columns2,
  NotebookPen,
  Table2,
  Wrench,
  type LucideIcon,
} from "lucide-react";

/** A page a grouped tab holds, and what it will show. */
export interface Page {
  slug: string;
  title: string;
  holds: string | null;
}

/** The Roth Conversions tool's page. */
export const ROTH_CONVERSIONS = "roth-conversions";

/** The SSA Benefits tool's page. */
export const SSA_BENEFITS = "ssa-benefits";

/** The tools that act on the plan as a whole, in the TUI's order. */
export const TOOLS: readonly Page[] = [
  {
    slug: ROTH_CONVERSIONS,
    title: "Roth Conversions",
    holds: "Conversion ladders searched bracket by bracket.",
  },
  {
    slug: SSA_BENEFITS,
    title: "SSA Benefits",
    holds: "Social Security claim ages ranked for the household.",
  },
  {
    slug: "monte-carlo",
    title: "Monte Carlo",
    holds: "The plan through random markets.",
  },
  {
    slug: "historical",
    title: "Historical",
    holds: "The plan from every historical start year.",
  },
];

/** The plan's editing domains, as the client names them. */
export const DOMAINS: readonly Page[] = domains().map((domain) => ({
  slug: domain.slug,
  title: domain.title,
  holds: domain.purpose,
}));

/** One of the five places to be: a page of its own, or a group of pages. */
export type Tab = { title: string; icon: LucideIcon } & (
  | { path: "/overview" | "/ledger" | "/compare" }
  | { path: "/tools/$page" | "/plan/$page"; pages: readonly Page[] }
);

/** A tab holding a group of pages. */
export type GroupTab = Extract<Tab, { pages: readonly Page[] }>;

export function isGroup(tab: Tab): tab is GroupTab {
  return "pages" in tab;
}

export const TABS: readonly Tab[] = [
  { title: "Overview", icon: ChartNoAxesColumn, path: "/overview" },
  { title: "Ledger", icon: Table2, path: "/ledger" },
  { title: "Compare", icon: Columns2, path: "/compare" },
  { title: "Tools", icon: Wrench, path: "/tools/$page", pages: TOOLS },
  { title: "Plan", icon: NotebookPen, path: "/plan/$page", pages: DOMAINS },
];

/** Whether `pathname` is `tab`'s page or one of its group's. */
export function isWithin(tab: Tab, pathname: string): boolean {
  const prefix = tab.path.replace("/$page", "");
  return pathname === prefix || pathname.startsWith(`${prefix}/`);
}

/** The page of `pages` at `slug`, or the router's not-found. */
export function pageOf(pages: readonly Page[], slug: string): Page {
  const page = pages.find((each) => each.slug === slug);
  // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router's own not-found
  if (!page) throw notFound();
  return page;
}
