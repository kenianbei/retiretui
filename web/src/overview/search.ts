import type { Chart } from "@wasm/retiretui_wasm.js";

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

/** Every chart the client names, so that one it adds or renames fails the build here. */
const CHARTS: Record<Chart, null> = {
  balances: null,
  "net-worth": null,
  "income-taxes": null,
  markets: null,
};

/** The chart shown where the address names none. */
export const FIRST_CHART: Chart = "balances";

/** The chart the address names, the first where it names none. */
export function chartOf(named: unknown): Chart {
  const isChart = typeof named === "string" && Object.hasOwn(CHARTS, named);
  return isChart ? (named as Chart) : FIRST_CHART;
}
