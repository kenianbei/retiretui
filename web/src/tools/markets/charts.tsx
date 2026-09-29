import type { MarketRuns, RunRow, Table } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  XAxis,
  YAxis,
} from "recharts";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  bandAreas,
  bandData,
  bandsConfig,
  PLOT_SIZE,
  SERIES,
} from "@/overview/bands";
import { Plot } from "@/overview/charts";
import { BASIS_LABEL } from "@/overview/view-words";

const SHORT = "var(--destructive)";
const DOLLARS = BASIS_LABEL.today;
const PERCENT_TICKS = [0, 25, 50, 75, 100];
const PERCENT = 100;
const BANDS = bandsConfig();

/** The spread of net worth across the runs, the highlighted run's line over it. */
function Bands({ found, run }: { found: MarketRuns; run: RunRow }) {
  const bands = useMemo(() => bandData(found.bands), [found]);
  const config: ChartConfig = useMemo(
    () => ({ ...BANDS, run: { label: run.cells[0], color: SERIES[1] } }),
    [run],
  );
  const data = useMemo(
    () => bands.map((year, at) => ({ ...year, run: run.net_worth[at] })),
    [bands, run],
  );
  return (
    <Plot config={config} data={data} label="Net worth across the runs">
      {bandAreas()}
      <Line
        isAnimationActive={false}
        dataKey="run"
        type="monotone"
        stroke="var(--color-run)"
        strokeWidth={2}
        dot={false}
      />
    </Plot>
  );
}

const byYearColumn = columnsFor<string[]>();

/** Net worth at each percentile, and the share still funded, year by year. */
function ByYear({ table }: { table: Table }) {
  const columns = useMemo(
    () =>
      table.columns.map((header, at) =>
        byYearColumn.display({
          id: String(at),
          header,
          meta: { isNumeric: at > 0 },
          cell: ({ row }) => row.original[at],
        }),
      ),
    [table],
  );
  return (
    <DataTable
      label={`Net worth by year at each percentile, ${DOLLARS}`}
      columns={columns}
      rows={table.rows}
      rowKey={(row) => row[0] ?? ""}
      isFirstPinned
      className="max-h-[28rem]"
    />
  );
}

const FUNDED: ChartConfig = {
  funded: { label: "Still funded", color: SERIES[0] },
};

/** The share of runs not yet short, year by year. */
function StillFunded({ found }: { found: MarketRuns }) {
  const data = useMemo(
    () =>
      found.bands.map((band) => ({
        year: band.year,
        funded: band.funded * PERCENT,
      })),
    [found],
  );
  return (
    <ChartContainer
      config={FUNDED}
      aria-label="Share of runs still funded"
      className={PLOT_SIZE}
    >
      <LineChart data={data} margin={{ left: 8, right: 8, top: 16 }}>
        <CartesianGrid vertical={false} />
        <XAxis dataKey="year" tickLine={false} minTickGap={24} />
        <YAxis
          width={48}
          domain={[0, 100]}
          ticks={PERCENT_TICKS}
          tickLine={false}
          axisLine={false}
          tickFormatter={(percent: number) => `${String(percent)}%`}
        />
        <ChartTooltip
          content={
            <ChartTooltipContent
              formatter={(value) => `${Number(value).toFixed(1)}%`}
            />
          }
        />
        <Line
          isAnimationActive={false}
          dataKey="funded"
          type="monotone"
          stroke="var(--color-funded)"
          strokeWidth={2}
          dot={false}
        />
      </LineChart>
    </ChartContainer>
  );
}

const ENDINGS: ChartConfig = {
  count: { label: "Runs", color: SERIES[0] },
};

/** How many runs end in each bucket, those that fell short first. */
function Endings({ found }: { found: MarketRuns }) {
  const data = useMemo(
    () =>
      found.endings.map((ending) => ({
        ...ending,
        fill: ending.is_short ? SHORT : "var(--color-count)",
      })),
    [found],
  );
  return (
    <ChartContainer
      config={ENDINGS}
      aria-label="What the runs end with"
      className={PLOT_SIZE}
    >
      <BarChart data={data} layout="vertical" margin={{ left: 8, right: 24 }}>
        <CartesianGrid horizontal={false} />
        <XAxis type="number" allowDecimals={false} tickLine={false} />
        <YAxis
          type="category"
          dataKey="label"
          width={96}
          tickLine={false}
          axisLine={false}
        />
        <ChartTooltip content={<ChartTooltipContent />} />
        <Bar
          isAnimationActive={false}
          dataKey="count"
          fill="var(--color-count)"
          label={{ position: "right", fontSize: 11 }}
        />
      </BarChart>
    </ChartContainer>
  );
}

/** The runs' spread, in each view the tool has. */
export function MarketCharts({
  found,
  run,
}: {
  found: MarketRuns;
  run: RunRow;
}) {
  const views = [
    {
      value: "bands",
      tab: "Net worth",
      title: "Net worth",
      unit: DOLLARS,
      chart: <Bands found={found} run={run} />,
    },
    ...(found.by_year
      ? [
          {
            value: "by-year",
            tab: "By year",
            title: "By year",
            unit: DOLLARS,
            chart: <ByYear table={found.by_year} />,
          },
        ]
      : []),
    {
      value: "funded",
      tab: "Funded",
      title: "Still funded",
      unit: "share of runs",
      chart: <StillFunded found={found} />,
    },
    {
      value: "endings",
      tab: "Endings",
      title: "Ends with",
      unit: DOLLARS,
      chart: <Endings found={found} />,
    },
  ];
  return (
    <Card className="gap-3 py-4">
      <Tabs defaultValue="bands" className="gap-3">
        <CardHeader className="px-4">
          <CardTitle className="sr-only">Charts</CardTitle>
          <TabsList className="w-full sm:w-fit">
            {views.map((view) => (
              <TabsTrigger
                key={view.value}
                value={view.value}
                title={view.title}
                className="max-md:touch-target h-8"
              >
                {view.tab}
              </TabsTrigger>
            ))}
          </TabsList>
        </CardHeader>
        <CardContent className="px-4">
          {views.map((view) => (
            <TabsContent
              key={view.value}
              value={view.value}
              className="space-y-2"
            >
              <p className="text-muted-foreground text-xs">
                {view.title} · {view.unit} ·{" "}
                {found.count.toLocaleString("en-US")} runs
              </p>
              {view.chart}
            </TabsContent>
          ))}
        </CardContent>
      </Tabs>
    </Card>
  );
}
