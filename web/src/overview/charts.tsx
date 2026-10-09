import { Link } from "@tanstack/react-router";
import {
  type Chart,
  type ChartMark,
  type ChartSeries,
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
import { BASIS_LABEL, metricTitle, VIEW_WORDS } from "@/overview/view-words";
import type { Basis } from "@/overview/words";
import { useMarkets } from "@/searches";
import { IN_PLACE, keptSearch } from "@/year/search";

const FOREGROUND = "var(--foreground)";
const MUTED = "var(--muted-foreground)";

interface ChartsProps {
  series: ChartSeries;
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
      dataKey={key}
      type="monotone"
      stroke={`var(--color-${key})`}
      strokeWidth={width}
      dot={false}
    />
  );
}

const classKey = (at: number) => `class${String(at)}`;

function Balances(props: ChartsProps) {
  const { series } = props;
  const [config, data] = useMemo(() => {
    const classes = series.classes.map(
      (label, at): [string, ChartConfig[string]] => [
        classKey(at),
        { label, color: SERIES[at % SERIES.length] },
      ],
    );
    const shown: ChartConfig = {
      ...Object.fromEntries(classes),
      net_worth: { label: metricTitle("net-worth"), color: FOREGROUND },
    };
    const rows = series.years.map((row) => ({
      year: row.year,
      net_worth: row.net_worth,
      ...Object.fromEntries(
        row.classes.map((amount, at) => [classKey(at), amount]),
      ),
    }));
    return [shown, rows] as const;
  }, [series]);
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={config}
      data={data}
      label={titleOf("balances")}
    >
      {series.classes.map((_, at) => (
        <Area
          isAnimationActive={false}
          key={classKey(at)}
          dataKey={classKey(at)}
          stackId="classes"
          type="monotone"
          fill={`var(--color-${classKey(at)})`}
          stroke={`var(--color-${classKey(at)})`}
          fillOpacity={0.5}
        />
      ))}
      {seriesLine("net_worth", 1)}
    </Plot>
  );
}

const NET_WORTH: ChartConfig = {
  net_worth: { label: metricTitle("net-worth"), color: SERIES[0] },
};

function NetWorth(props: ChartsProps) {
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={NET_WORTH}
      data={props.series.years}
      label={titleOf("net-worth")}
    >
      {seriesLine("net_worth")}
    </Plot>
  );
}

const INCOME_AND_TAX: ChartConfig = {
  income: { label: metricTitle("income"), color: SERIES[1] },
  taxes: { label: metricTitle("taxes"), color: SERIES[2] },
};

function IncomeAndTax(props: ChartsProps) {
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={INCOME_AND_TAX}
      data={props.series.years}
      label={titleOf("income-taxes")}
    >
      {seriesLine("income")}
      {seriesLine("taxes")}
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
  balances: Balances,
  "net-worth": NetWorth,
  "income-taxes": IncomeAndTax,
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
