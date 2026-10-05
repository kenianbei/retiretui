import { type YearSearch, yearSearch } from "@/year/search";

/** What the Overview holds in its address: the chart on show, beside the
 * year, the basis and the held claims it carries on to the pages it leads to. */
export interface OverviewSearch extends YearSearch {
  /** The chart on show; the first where none. */
  chart?: string;
}

/** The Overview's search params from whatever the address holds. */
export function overviewSearch(
  search: Record<string, unknown>,
): OverviewSearch {
  const chart = typeof search.chart === "string" ? search.chart : "";
  return { ...yearSearch(search), ...(chart !== "" && { chart }) };
}

/** The Overview's charts in the order they are turned through. */
export const CHART_KEYS = [
  "balances",
  "net-worth",
  "income",
  "markets",
] as const;

export type ChartKey = (typeof CHART_KEYS)[number];

/** The chart shown where the address names none. */
export const [FIRST_CHART] = CHART_KEYS;

/** The chart the address names, the first where it names none. */
export function chartOf(named: unknown): ChartKey {
  return CHART_KEYS.find((key) => key === named) ?? FIRST_CHART;
}
