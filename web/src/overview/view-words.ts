import { compareWords, viewWords, type Metric } from "@wasm/retiretui_wasm.js";

import type { Basis } from "@/overview/words";

/** What the Overview, the Ledger and Compare call what they show, in the client's words. */
export const VIEW_WORDS = viewWords();

/** The dollars a figure is shown in, as every surface says them. */
export const BASIS_LABEL: Record<Basis, string> = VIEW_WORDS.basis;

const METRICS = compareWords().metrics;

/** What a metric is called, as every surface calls it. */
export function metricTitle(metric: Metric): string {
  return METRICS.find((each) => each.key === metric)?.title ?? metric;
}
