import type { HistoryChart } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";
import { Line } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { ChartConfig } from "@/components/ui/chart";
import { SERIES } from "@/overview/bands";
import { Plot } from "@/overview/charts";

/** The keys a history's two lines are plotted under. */
const KEYS = ["first", "second"] as const;

/** One of the year's money panes across every year: its two figures as lines, the year shown marked, a click on a year showing it. */
export function History({
  chart,
  year,
  onYear,
}: {
  chart: HistoryChart;
  year: number;
  onYear: (year: number) => void;
}) {
  const config: ChartConfig = useMemo(
    () => ({
      first: { label: chart.lines[0], color: SERIES[0] },
      second: { label: chart.lines[1], color: SERIES[1] },
    }),
    [chart],
  );
  const data = useMemo(
    () =>
      chart.years.map(({ year, figures: [first, second] }) => ({
        year,
        first,
        second,
      })),
    [chart],
  );
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="px-4">
        <CardTitle>
          <h3>{chart.title}</h3>
        </CardTitle>
      </CardHeader>
      <CardContent className="@container px-4">
        <Plot
          config={config}
          data={data}
          label={chart.title}
          year={year}
          onYear={onYear}
        >
          {KEYS.map((key) => (
            <Line
              key={key}
              isAnimationActive={false}
              dataKey={key}
              type="monotone"
              stroke={`var(--color-${key})`}
              strokeWidth={2}
              dot={false}
            />
          ))}
        </Plot>
      </CardContent>
    </Card>
  );
}
