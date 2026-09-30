import type { MarketRuns, RunRow, Table } from "@wasm/retiretui_wasm.js";
import { memo, useMemo } from "react";
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
import { ChartSection } from "@/components/chart-section";
import { DataTable } from "@/components/data-table";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
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
    () => ({ ...BANDS, run: { label: run.cells[0], color: SERIES[0] } }),
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
const ByYear = memo(function ByYear({ table }: { table: Table }) {
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
});

const FUNDED: ChartConfig = {
  funded: { label: "Still funded", color: SERIES[0] },
};

/** The share of runs not yet short, year by year. */
const StillFunded = memo(function StillFunded({
  found,
}: {
  found: MarketRuns;
}) {
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
      role="img"
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
});

const ENDINGS: ChartConfig = {
  count: { label: "Runs", color: SERIES[0] },
};

/** How many runs end in each bucket, those that fell short first. */
const Endings = memo(function Endings({ found }: { found: MarketRuns }) {
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
      role="img"
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
});

/** The runs' spread, each view a section of its own. */
export function MarketCharts({
  found,
  run,
}: {
  found: MarketRuns;
  run: RunRow;
}) {
  const runs = `${found.count.toLocaleString("en-US")} runs`;
  return (
    <div className="space-y-6">
      <ChartSection title="Net worth" unit={`${DOLLARS} · ${runs}`}>
        <Bands found={found} run={run} />
      </ChartSection>
      <div className="grid items-start gap-6 @4xl/page:grid-cols-2">
        <ChartSection title="Still funded" unit={`share of runs · ${runs}`}>
          <StillFunded found={found} />
        </ChartSection>
        <ChartSection title="Ends with" unit={`${DOLLARS} · ${runs}`}>
          <Endings found={found} />
        </ChartSection>
      </div>
      {found.by_year && (
        <ChartSection title="By year" unit={`${DOLLARS} · ${runs}`}>
          <ByYear table={found.by_year} />
        </ChartSection>
      )}
    </div>
  );
}
