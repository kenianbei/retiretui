import type { CompareWords, Metric, YearFigure } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";
import { Line } from "recharts";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { ChartConfig } from "@/components/ui/chart";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn, INPUT } from "@/lib/utils";
import { SERIES } from "@/overview/bands";
import { Plot } from "@/overview/charts";

/** A plan's metric year by year, as the views show it. */
export interface Charted {
  name: string;
  /** Its figures, or why there are none. */
  figures: YearFigure[] | string;
  /** Drawn along zero: the baseline, the others read as its differences. */
  isAlongZero: boolean;
}

export type ViewName = "chart" | "table";

interface ViewsProps {
  plans: Charted[];
  words: CompareWords;
  metric: Metric;
  view: ViewName;
  /** What the figures are in, and whether they are differences. */
  caption: string;
  year: number;
  onYear: (year: number) => void;
  onView: (view: ViewName) => void;
  onMetric: (metric: Metric) => void;
}

type YearRow = { year: number } & Record<string, number | string>;

const seriesKey = (at: number) => `plan${String(at)}`;

/** A row per year any plan reaches, each plan's `valueOf` its figure under its key. */
function rowsByYear(
  plans: readonly Charted[],
  valueOf: (plan: Charted, shown: YearFigure) => number | string,
): YearRow[] {
  const years = new Map<number, YearRow>();
  plans.forEach((plan, at) => {
    if (typeof plan.figures === "string") return;
    for (const shown of plan.figures) {
      const row = years.get(shown.year) ?? { year: shown.year };
      row[seriesKey(at)] = valueOf(plan, shown);
      years.set(shown.year, row);
    }
  });
  return [...years.values()].sort((one, other) => one.year - other.year);
}

/** One line per plan through the years any of them reach. */
function PlansChart({ plans, year, onYear, caption }: ViewsProps) {
  const config = useMemo<ChartConfig>(
    () =>
      Object.fromEntries(
        plans.map((plan, at) => [
          seriesKey(at),
          { label: plan.name, color: SERIES[at % SERIES.length] },
        ]),
      ),
    [plans],
  );
  const data = useMemo(
    () =>
      rowsByYear(plans, (plan, shown) => (plan.isAlongZero ? 0 : shown.amount)),
    [plans],
  );
  return (
    <Plot
      config={config}
      data={data}
      label={caption}
      year={year}
      onYear={onYear}
    >
      {plans.map((_, at) => (
        <Line
          key={seriesKey(at)}
          dataKey={seriesKey(at)}
          stroke={`var(--color-${seriesKey(at)})`}
          strokeWidth={2}
          dot={false}
          isAnimationActive={false}
        />
      ))}
    </Plot>
  );
}

/** A row per year any plan reaches, a column per plan. */
function PlansTable({ plans, words, year, onYear, caption }: ViewsProps) {
  const { columns, rows } = useMemo(() => {
    const column = columnsFor<YearRow>();
    return {
      columns: [
        column.display({
          id: "year",
          header: "Year",
          meta: { isNumeric: false },
          cell: ({ row }) => row.original.year,
        }),
        ...plans.map((plan, at) =>
          column.display({
            id: seriesKey(at),
            header: plan.name,
            meta: { isNumeric: true },
            cell: ({ row }) => row.original[seriesKey(at)] ?? words.unreached,
          }),
        ),
      ],
      rows: rowsByYear(plans, (_, shown) => shown.figure),
    };
  }, [plans, words]);
  return (
    <DataTable
      label={caption}
      columns={columns}
      rows={rows}
      rowKey={(row) => String(row.year)}
      isSelected={(row) => row.year === year}
      onSelect={(row) => {
        onYear(row.year);
      }}
      isFirstPinned
      className="max-h-[28rem]"
    />
  );
}

/** The plans' metric year by year, charted or tabled, and the metric picked. */
export function Views(props: ViewsProps) {
  const { words, metric, view, caption, onView, onMetric } = props;
  return (
    <Card className="min-w-0 gap-3 py-4">
      <Tabs
        value={view}
        onValueChange={(value) => {
          onView(value === "table" ? "table" : "chart");
        }}
        className="gap-3"
      >
        <CardHeader className="flex flex-wrap items-center justify-between gap-2 px-4">
          <CardTitle className="sr-only">By year</CardTitle>
          <TabsList>
            <TabsTrigger value="chart" className="h-8">
              Chart
            </TabsTrigger>
            <TabsTrigger value="table" className="h-8">
              By year
            </TabsTrigger>
          </TabsList>
          <label className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">Metric</span>
            <select
              value={metric}
              onChange={(event) => {
                const picked = words.metrics.find(
                  (each) => each.key === event.target.value,
                );
                if (picked) onMetric(picked.key);
              }}
              className={cn(INPUT, "h-8 w-auto")}
            >
              {words.metrics.map((each) => (
                <option key={each.key} value={each.key}>
                  {each.title}
                </option>
              ))}
            </select>
          </label>
        </CardHeader>
        <CardContent className="space-y-2 px-4">
          <p className="text-muted-foreground text-xs">{caption}</p>
          <TabsContent value="chart">
            <PlansChart {...props} />
          </TabsContent>
          <TabsContent value="table">
            <PlansTable {...props} />
          </TabsContent>
        </CardContent>
      </Tabs>
    </Card>
  );
}
