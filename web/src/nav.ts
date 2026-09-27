import { notFound } from "@tanstack/react-router";
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
  holds: string;
}

/** The tools that act on the plan as a whole, in the TUI's order. */
export const TOOLS: readonly Page[] = [
  {
    slug: "roth-conversions",
    title: "Roth Conversions",
    holds: "Conversion ladders searched bracket by bracket.",
  },
  {
    slug: "ssa-benefits",
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

/** The plan's editing domains, in the TUI's order. */
export const DOMAINS: readonly Page[] = [
  {
    slug: "accounts",
    title: "Accounts",
    holds: "What the household holds, and how each is taxed.",
  },
  {
    slug: "income",
    title: "Income",
    holds: "Salaries, pensions, Social Security and other income.",
  },
  {
    slug: "expenses",
    title: "Expenses",
    holds: "What the household spends, and when.",
  },
  {
    slug: "cliffs",
    title: "Cliffs",
    holds: "Income limits the plan should stay under.",
  },
  {
    slug: "transfers",
    title: "Transfers",
    holds: "Scheduled moves between accounts.",
  },
  { slug: "conversions", title: "Conversions", holds: "Roth conversions." },
  {
    slug: "contributions",
    title: "Contributions",
    holds: "What is paid into each account.",
  },
  {
    slug: "events",
    title: "Events",
    holds: "Named moments other items start or stop on.",
  },
  {
    slug: "people",
    title: "People",
    holds: "The household's people and their earnings records.",
  },
  {
    slug: "residency",
    title: "Residency",
    holds: "Where the household lives, and from when.",
  },
  {
    slug: "household",
    title: "Household",
    holds: "Filing status and Medicare.",
  },
  {
    slug: "settings",
    title: "Settings",
    holds: "The plan's years and inflation.",
  },
  { slug: "market", title: "Market", holds: "What the market tools assume." },
];

/** One of the five places to be: a page of its own, or a group of pages. */
export type Tab = { title: string; icon: LucideIcon } & (
  | { path: "/overview" | "/ledger" | "/compare" }
  | { path: "/tools/$page" | "/plan/$page"; pages: readonly Page[] }
);

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
