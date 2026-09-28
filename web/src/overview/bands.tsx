import {
  bandPercentiles,
  percentileLabel,
  type Band,
} from "@wasm/retiretui_wasm.js";

import { Area } from "recharts";

import type { ChartConfig } from "@/components/ui/chart";

/** The colour roles a chart's series take, in turn. */
export const SERIES = [1, 2, 3, 4, 5].map((at) => `var(--chart-${String(at)})`);

export const PLOT_SIZE = "aspect-[4/3] w-full sm:aspect-[5/2]";

/** The bands' spans and the median, named by their percentiles. */
export function bandsConfig(): ChartConfig {
  const [low, lower, median, upper, high] = bandPercentiles();
  const span = (from: number, to: number) =>
    `${percentileLabel(from)} – ${percentileLabel(to)}`;
  return {
    outer: { label: span(low, high), color: SERIES[0] },
    inner: { label: span(lower, upper), color: SERIES[0] },
    median: { label: percentileLabel(median), color: SERIES[0] },
  };
}

/** Each year's bands as the chart plots them. */
export function bandData(bands: Band[]) {
  return bands.map((band) => {
    const [low, lower, median, upper, high] = band.net_worth;
    return {
      year: band.year,
      outer: [low, high],
      inner: [lower, upper],
      median,
    };
  });
}

/** The outer and inner bands, shaded lighter and darker. */
export function bandAreas() {
  return [
    <Area
      key="outer"
      isAnimationActive={false}
      dataKey="outer"
      type="monotone"
      fill="var(--color-outer)"
      fillOpacity={0.15}
      stroke="none"
    />,
    <Area
      key="inner"
      isAnimationActive={false}
      dataKey="inner"
      type="monotone"
      fill="var(--color-inner)"
      fillOpacity={0.35}
      stroke="none"
    />,
  ];
}
