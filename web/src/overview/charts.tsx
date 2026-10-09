import { Link } from "@tanstack/react-router";
import {
  type Chart,
  type Charted,
  type ChartMark,
  compactMoney,
  money,
  signedMoney,
} from "@wasm/retiretui_wasm.js";
import { type ReactNode, useId, useMemo } from "react";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceLine,
  XAxis,
  YAxis,
} from "recharts";

import { ChartSection } from "@/components/chart-section";
import {
  type ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  bandAreas,
  bandData,
  bandsConfig,
  PLOT_SIZE,
  SERIES,
} from "@/overview/bands";
import { FIRST_CHART } from "@/overview/search";
import { BASIS_LABEL, VIEW_WORDS } from "@/overview/view-words";
import type { Basis } from "@/overview/words";
import { useMarkets } from "@/searches";
import { IN_PLACE, keptSearch } from "@/year/search";

const FOREGROUND = "var(--foreground)";
const MUTED = "var(--muted-foreground)";

interface ChartsProps {
  /** What the chart shown draws of the plan's own projection. */
  series: Charted;
  basis: Basis;
  /** The plan's text for its market runs; none while it has issues. */
  plan: string | null;
  /** What a click on a year does with it. */
  onYear: (year: number) => void;
}

/** The year a plotted row is of, as a tooltip heads it. */
function yearOf(row: unknown): string {
  const year = (row as { year?: unknown } | undefined)?.year;
  return typeof year === "number" ? String(year) : "";
}

/** The year under the pointer over each series' figure there, signed where
 * the plot is of differences. */
function tooltip(config: ChartConfig, isDifference: boolean) {
  const said = isDifference ? signedMoney : money;
  return (
    <ChartTooltip
      content={
        <ChartTooltipContent
          labelFormatter={(_, [first]) => yearOf(first?.payload)}
          formatter={(value, name) => (
            <div className="flex w-full justify-between gap-4">
              <span className="text-muted-foreground">
                {config[String(name)]?.label}
              </span>
              <span className="tabular-nums">
                {Array.isArray(value)
                  ? value.map((each) => said(Number(each))).join(" – ")
                  : said(Number(value))}
              </span>
            </div>
          )}
        />
      }
    />
  );
}

interface PlotProps {
  config: ChartConfig;
  data: object[];
  label: string;
  /** The years marked, each salary's end. */
  marks?: readonly ChartMark[];
  /** Whether its figures are differences from a baseline. */
  isDifference?: boolean;
  year?: number;
  onYear?: (year: number) => void;
  children: ReactNode;
}

/** The key to a mark: the dashed line the plot draws it as. */
function MarkKey() {
  return (
    <svg aria-hidden width="16" height="8" className="shrink-0">
      <line
        x1="0"
        x2="16"
        y1="4"
        y2="4"
        stroke={MUTED}
        strokeDasharray="2 4"
        strokeWidth="1.5"
      />
    </svg>
  );
}

/**
 * A plot of years across and dollars up, `marks` marked and listed under
 * it and the year shown marked where there is one, a click giving the
 * year under it to `onYear` where it takes one. It reads as one image
 * named by `label`; its table is the keyboard's way to a year.
 */
export function Plot({
  config,
  data,
  label,
  marks = [],
  isDifference = false,
  year,
  onYear,
  children,
}: PlotProps) {
  const marksId = useId();
  return (
    <div className="space-y-2">
      <ChartContainer
        config={config}
        role="img"
        aria-label={label}
        aria-describedby={marks.length > 0 ? marksId : undefined}
        className={PLOT_SIZE}
      >
        <ComposedChart
          accessibilityLayer={false}
          data={data}
          margin={{ left: 8, right: 8, top: 16 }}
          // A tap's move and click come together; the click must see the move.
          throttledEvents={[]}
          onClick={(state) => {
            const clicked = Number(state.activeLabel);
            if (onYear && Number.isInteger(clicked)) onYear(clicked);
          }}
          className={onYear && "cursor-pointer"}
        >
          <CartesianGrid vertical={false} />
          <XAxis dataKey="year" tickLine={false} minTickGap={24} />
          <YAxis
            width={64}
            tickLine={false}
            axisLine={false}
            tickFormatter={(amount: number) => compactMoney(amount)}
          />
          {tooltip(config, isDifference)}
          <ChartLegend content={<ChartLegendContent />} />
          {children}
          {marks.map((mark) => (
            <ReferenceLine
              key={mark.label}
              x={mark.year}
              stroke={MUTED}
              strokeDasharray="2 4"
            />
          ))}
          {year !== undefined && <ReferenceLine x={year} stroke={FOREGROUND} />}
        </ComposedChart>
      </ChartContainer>
      {marks.length > 0 && (
        <p
          id={marksId}
          className="text-muted-foreground flex flex-wrap items-center gap-x-2 text-xs"
        >
          <MarkKey />
          {marks.map((mark) => mark.label).join(" · ")}
        </p>
      )}
    </div>
  );
}

function seriesLine(key: string, width = 2) {
  return (
    <Line
      isAnimationActive={false}
      key={key}
      dataKey={key}
      type="monotone"
      stroke={`var(--color-${key})`}
      strokeWidth={width}
      dot={false}
    />
  );
}

const stackKey = (at: number) => `stack${String(at)}`;
const lineKey = (at: number) => `line${String(at)}`;

/** The colour each chart draws its lines in, by their place; a line over a stack is in ink. */
const LINE_COLOURS: Record<
  Exclude<Chart, "markets">,
  (string | undefined)[]
> = {
  balances: [FOREGROUND],
  "net-worth": [SERIES[0]],
  "income-taxes": [SERIES[1], SERIES[2]],
};

/** What the plan's own projection charts: its stack, if any, under its lines. */
function Lines(props: ChartsProps & { chart: Exclude<Chart, "markets"> }) {
  const { series, chart } = props;
  const [config, data] = useMemo(() => {
    const keyed = [
      ...series.stacked.map((line, at) => ({
        key: stackKey(at),
        line,
        color: SERIES[at % SERIES.length],
      })),
      ...series.lines.map((line, at) => ({
        key: lineKey(at),
        line,
        color: LINE_COLOURS[chart][at],
      })),
    ];
    const shown: ChartConfig = Object.fromEntries(
      keyed.map(({ key, line, color }) => [key, { label: line.label, color }]),
    );
    const years = keyed[0]?.line.points ?? [];
    const rows = years.map(([year], at) => ({
      year,
      ...Object.fromEntries(
        keyed.map(({ key, line }) => [key, line.points[at]?.[1]]),
      ),
    }));
    return [shown, rows] as const;
  }, [series, chart]);
  const isOverStack = series.stacked.length > 0;
  return (
    <Plot
      {...props}
      marks={series.marks}
      config={config}
      data={data}
      label={titleOf(chart)}
    >
      {series.stacked.map((_, at) => (
        <Area
          isAnimationActive={false}
          key={stackKey(at)}
          dataKey={stackKey(at)}
          stackId="stacked"
          type="monotone"
          fill={`var(--color-${stackKey(at)})`}
          stroke={`var(--color-${stackKey(at)})`}
          fillOpacity={0.5}
        />
      ))}
      {series.lines.map((_, at) =>
        seriesLine(lineKey(at), isOverStack ? 1 : 2),
      )}
    </Plot>
  );
}

function MarketRuns(props: ChartsProps) {
  if (props.plan === null) {
    return (
      <p className="text-muted-foreground py-8 text-sm">
        The market runs show once the plan&apos;s issues are fixed.
      </p>
    );
  }
  return <Bands {...props} plan={props.plan} />;
}

function Bands(props: ChartsProps & { plan: string }) {
  const markets = useMarkets("monteCarlo", props.plan, true);
  const config = useMemo(() => bandsConfig(), []);
  const data = useMemo(
    () => (markets.data ? bandData(markets.data.bands) : []),
    [markets.data],
  );
  if (markets.error) {
    return (
      <p className="text-muted-foreground py-8 text-sm">
        {markets.error.message}
      </p>
    );
  }
  if (!markets.data || markets.isPlaceholderData) {
    return <Skeleton className={PLOT_SIZE} />;
  }
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={config}
      data={data}
      label={titleOf("markets")}
    >
      {bandAreas()}
      {seriesLine("median")}
    </Plot>
  );
}

/** What draws each chart the client names. */
const DRAWN: Record<Chart, (props: ChartsProps) => ReactNode> = {
  balances: (props) => <Lines {...props} chart="balances" />,
  "net-worth": (props) => <Lines {...props} chart="net-worth" />,
  "income-taxes": (props) => <Lines {...props} chart="income-taxes" />,
  markets: MarketRuns,
};

/** What the client titles `chart`. */
function titleOf(chart: Chart): string {
  return VIEW_WORDS.charts.find(([key]) => key === chart)?.[1] ?? chart;
}

/** The charts as tabs, the one on show marked. */
function ChartTabs({ shown }: { shown: Chart }) {
  return (
    <nav aria-label="Chart" className="flex flex-wrap gap-1">
      {VIEW_WORDS.charts.map(([key, title]) => (
        <Link
          key={key}
          to="/overview"
          search={(kept) => ({
            ...kept,
            chart: key === FIRST_CHART ? undefined : key,
          })}
          {...IN_PLACE}
          aria-current={key === shown ? "true" : undefined}
          className={cn(
            "focus-visible:ring-ring/50 rounded-md px-2.5 py-1 text-sm focus-visible:ring-[3px] focus-visible:outline-none",
            key === shown
              ? "bg-accent text-accent-foreground font-medium"
              : "text-muted-foreground hover:bg-muted",
          )}
        >
          {title}
        </Link>
      ))}
    </nav>
  );
}

/**
 * What the plan holds and earns year by year, or how random markets spread
 * it: one chart at a time, turned by its tabs, a click on a year opening
 * it in the Ledger, which is every chart's table.
 */
export function PlanChart({ chart, ...props }: ChartsProps & { chart: Chart }) {
  const Drawn = DRAWN[chart];
  const unit = BASIS_LABEL[chart === "markets" ? "today" : props.basis];
  return (
    <ChartSection
      title={titleOf(chart)}
      unit={`${unit} · click a year to open it in the Ledger`}
      controls={<ChartTabs shown={chart} />}
    >
      <Drawn {...props} />
      <Link
        to="/ledger"
        search={(kept) => keptSearch(kept, ["year", "basis", "held"])}
        className="text-primary inline-block text-sm underline-offset-4 hover:underline"
      >
        Every year in the Ledger
      </Link>
    </ChartSection>
  );
}
