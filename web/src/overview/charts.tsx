import {
  compactMoney,
  type ChartMark,
  type ChartSeries,
} from "@wasm/retiretui_wasm.js";
import { useMemo, type ReactNode } from "react";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceLine,
  XAxis,
  YAxis,
} from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { BASIS_LABEL, dollars, type Basis } from "@/overview/words";
import { bandData, bandsConfig, PLOT_SIZE, SERIES } from "@/overview/bands";
import { useMarkets } from "@/searches";

const FOREGROUND = "var(--foreground)";
const MUTED = "var(--muted-foreground)";

interface ChartsProps {
  series: ChartSeries;
  basis: Basis;
  /** The plan's text for its market runs; none while it has issues. */
  plan: string | null;
  year: number | undefined;
  onYear: (year: number) => void;
}

function tooltip(config: ChartConfig) {
  return (
    <ChartTooltip
      content={
        <ChartTooltipContent
          formatter={(value, name) => (
            <div className="flex w-full justify-between gap-4">
              <span className="text-muted-foreground">
                {config[String(name)]?.label}
              </span>
              <span className="tabular-nums">
                {Array.isArray(value)
                  ? value.map((each) => dollars(Number(each))).join(" – ")
                  : dollars(Number(value))}
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
  year?: number;
  onYear?: (year: number) => void;
  children: ReactNode;
}

/**
 * A plot of years across and dollars up, the year shown and `marks` marked,
 * a click choosing the year under it where `onYear` takes one.
 */
export function Plot({
  config,
  data,
  label,
  marks = [],
  year,
  onYear,
  children,
}: PlotProps) {
  return (
    <ChartContainer config={config} aria-label={label} className={PLOT_SIZE}>
      <ComposedChart
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
        {tooltip(config)}
        <ChartLegend content={<ChartLegendContent />} />
        {children}
        {marks.map((mark) => (
          <ReferenceLine
            key={mark.label}
            x={mark.year}
            stroke={MUTED}
            strokeDasharray="2 4"
            label={{
              value: mark.label,
              position: "insideTopLeft",
              fontSize: 11,
            }}
          />
        ))}
        {year !== undefined && <ReferenceLine x={year} stroke={FOREGROUND} />}
      </ComposedChart>
    </ChartContainer>
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
      net_worth: { label: "Net worth", color: FOREGROUND },
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
      label="Balances by tax treatment"
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
  net_worth: { label: "Net worth", color: SERIES[0] },
};

function NetWorth(props: ChartsProps) {
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={NET_WORTH}
      data={props.series.years}
      label="Net worth"
    >
      {seriesLine("net_worth")}
    </Plot>
  );
}

const INCOME_AND_TAX: ChartConfig = {
  income: { label: "Income", color: SERIES[1] },
  taxes: { label: "Taxes", color: SERIES[3] },
};

function IncomeAndTax(props: ChartsProps) {
  return (
    <Plot
      {...props}
      marks={props.series.marks}
      config={INCOME_AND_TAX}
      data={props.series.years}
      label="Income against taxes"
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
      label="Net worth through random markets"
    >
      <Area
        isAnimationActive={false}
        dataKey="outer"
        type="monotone"
        fill="var(--color-outer)"
        fillOpacity={0.15}
        stroke="none"
      />
      <Area
        isAnimationActive={false}
        dataKey="inner"
        type="monotone"
        fill="var(--color-inner)"
        fillOpacity={0.35}
        stroke="none"
      />
      {seriesLine("median")}
    </Plot>
  );
}

const TABS = [
  { value: "balances", title: "Balances", Chart: Balances },
  { value: "net-worth", title: "Net worth", Chart: NetWorth },
  { value: "income", title: "Income & tax", Chart: IncomeAndTax },
  { value: "markets", title: "Market runs", Chart: MarketRuns },
] as const;

/** What the plan holds and earns year by year, and how random markets spread it. */
export function Charts(props: ChartsProps) {
  const unit = (value: string) =>
    BASIS_LABEL[value === "markets" ? "today" : props.basis];
  return (
    <Card className="gap-3 py-4">
      <Tabs defaultValue="balances" className="gap-3">
        <CardHeader className="flex flex-wrap items-center justify-between gap-2 px-4">
          <CardTitle className="sr-only">Charts</CardTitle>
          <TabsList className="grid w-full grid-cols-2 group-data-[orientation=horizontal]/tabs:h-auto sm:flex sm:w-fit">
            {TABS.map((tab) => (
              <TabsTrigger key={tab.value} value={tab.value} className="h-8">
                {tab.title}
              </TabsTrigger>
            ))}
          </TabsList>
        </CardHeader>
        <CardContent className="px-4">
          {TABS.map(({ value, title, Chart }) => (
            <TabsContent key={value} value={value} className="space-y-2">
              <p className="text-muted-foreground text-xs">
                {title} · {unit(value)} · click a year to show it
              </p>
              <Chart {...props} />
            </TabsContent>
          ))}
        </CardContent>
      </Tabs>
    </Card>
  );
}
