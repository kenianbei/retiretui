import { type Band, bandWords } from "@wasm/retiretui_wasm.js";

import { Area } from "recharts";

import type { ChartConfig } from "@/components/ui/chart";

/** The colour roles a chart's series take, in turn. */
export const SERIES = [1, 2, 3, 4, 5].map((at) => `var(--chart-${String(at)})`);

/** A chart's shape by the width it is given, not the window's; its section is the container. */
export const PLOT_SIZE =
  "aspect-[4/3] w-full @xl:aspect-[2/1] @3xl:aspect-[5/2] @6xl:aspect-[3/1]";

/** The neutral the runs' spread is drawn in, so the plan's own line is the one in ink. */
const SPREAD = "var(--muted-foreground)";

/** The spread at a strength, so a legend's swatch shows the fill the chart draws. */
const spreadAt = (percent: number) =>
  `color-mix(in oklab, ${SPREAD} ${String(percent)}%, transparent)`;

const { outer, inner, median } = bandWords();

/** The bands' spans and the median, as every surface names them. */
export function bandsConfig(): ChartConfig {
  return {
    outer: { label: outer.label, color: spreadAt(18) },
    inner: { label: inner.label, color: spreadAt(40) },
    median: { label: median.label, color: SPREAD },
  };
}

/** Each year's bands as the chart plots them. */
export function bandData(bands: Band[]) {
  return bands.map(({ year, net_worth: at }) => ({
    year,
    outer: [at[outer.low], at[outer.high]],
    inner: [at[inner.low], at[inner.high]],
    median: at[median.low],
  }));
}

/** The outer and inner bands, the inner drawn darker over the outer. */
export function bandAreas() {
  return [
    <Area
      key="outer"
      isAnimationActive={false}
      dataKey="outer"
      type="monotone"
      fill="var(--color-outer)"
      stroke="none"
    />,
    <Area
      key="inner"
      isAnimationActive={false}
      dataKey="inner"
      type="monotone"
      fill="var(--color-inner)"
      stroke="none"
    />,
  ];
}
