import {
  compactMoney,
  money,
  percentileLabel,
  type Band,
  type ChartSeries,
} from "@wasm/retiretui_wasm.js";
import type { ReactNode } from "react";
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
import { BASIS_LABEL, type Basis } from "@/overview/words";
import { useMonteCarlo } from "@/searches";

/** The colour roles a chart's series take, in turn. */
const SERIES = [1, 2, 3, 4, 5].map((at) => `var(--chart-${String(at)})`);
const FOREGROUND = "var(--foreground)";
const MUTED = "var(--muted-foreground)";
/** The percentiles a band spans, as `Band.net_worth` holds them: 10, 25, 50, 75, 90. */
const PERCENTILES = [10, 25, 50, 75, 90] as const;

interface Marks {
  year: number | undefined;
  marks: ChartSeries["marks"];
}

/** The year shown and where each salary ends, as lines across the plot. */
function markLines({ year, marks }: Marks) {
  return [
    ...marks.map((mark) => (
      <ReferenceLine
        key={mark.label}
        x={mark.year}
        stroke={MUTED}
        strokeDasharray="2 4"
        label={{ value: mark.label, position: "insideTopLeft", fontSize: 11 }}
      />
    )),
    year !== undefined && (
      <ReferenceLine key="year" x={year} stroke={FOREGROUND} />
    ),
  ];
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
                  ? value.map((each) => money(Number(each))).join(" – ")
                  : money(Number(value))}
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
  onYear: (year: number) => void;
  children: ReactNode;
}

/** A plot of years across and dollars up, a click choosing the year under it. */
function Plot({ config, data, label, onYear, children }: PlotProps) {
  return (
    <ChartContainer
      config={config}
      aria-label={label}
      className="aspect-[4/3] w-full sm:aspect-[5/2]"
    >
      <ComposedChart
        data={data}
        margin={{ left: 8, right: 8, top: 16 }}
        // A tap's move and click come together; the click must see the move.
        throttledEvents={[]}
        onClick={(state) => {
          const year = Number(state.activeLabel);
          if (Number.isInteger(year)) onYear(year);
        }}
        className="cursor-pointer"
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
      </ComposedChart>
    </ChartContainer>
  );
}

interface ChartsProps extends Marks {
  series: ChartSeries;
  basis: Basis;
  /** The plan's text for its market runs; none while it has issues. */
  plan: string | null;
  onYear: (year: number) => void;
}

function Balances({ series, year, marks, onYear }: ChartsProps) {
  const config: ChartConfig = {
    net_worth: { label: "Net worth", color: FOREGROUND },
  };
  series.classes.forEach((label, at) => {
    config[`class${String(at)}`] = {
      label,
      color: SERIES[at % SERIES.length],
    };
  });
  const data = series.years.map((row) => ({
    year: row.year,
    net_worth: row.net_worth,
    ...Object.fromEntries(
      row.classes.map((amount, at) => [`class${String(at)}`, amount]),
    ),
  }));
  return (
    <Plot
      config={config}
      data={data}
      label="Balances by tax treatment"
      onYear={onYear}
    >
      {series.classes.map((_, at) => {
        const key = `class${String(at)}`;
        return (
          <Area
            isAnimationActive={false}
            key={key}
            dataKey={key}
            stackId="classes"
            type="monotone"
            fill={`var(--color-${key})`}
            stroke={`var(--color-${key})`}
            fillOpacity={0.5}
          />
        );
      })}
      <Line
        isAnimationActive={false}
        dataKey="net_worth"
        type="monotone"
        stroke="var(--color-net_worth)"
        dot={false}
      />
      {markLines({ year, marks })}
    </Plot>
  );
}

function NetWorth({ series, year, marks, onYear }: ChartsProps) {
  const config: ChartConfig = {
    net_worth: { label: "Net worth", color: SERIES[0] },
  };
  return (
    <Plot config={config} data={series.years} label="Net worth" onYear={onYear}>
      <Line
        isAnimationActive={false}
        dataKey="net_worth"
        type="monotone"
        stroke="var(--color-net_worth)"
        strokeWidth={2}
        dot={false}
      />
      {markLines({ year, marks })}
    </Plot>
  );
}

function IncomeAndTax({ series, year, marks, onYear }: ChartsProps) {
  const config: ChartConfig = {
    income: { label: "Income", color: SERIES[1] },
    taxes: { label: "Taxes", color: SERIES[3] },
  };
  return (
    <Plot
      config={config}
      data={series.years}
      label="Income against taxes"
      onYear={onYear}
    >
      <Line
        isAnimationActive={false}
        dataKey="income"
        type="monotone"
        stroke="var(--color-income)"
        strokeWidth={2}
        dot={false}
      />
      <Line
        isAnimationActive={false}
        dataKey="taxes"
        type="monotone"
        stroke="var(--color-taxes)"
        strokeWidth={2}
        dot={false}
      />
      {markLines({ year, marks })}
    </Plot>
  );
}

function bandData(bands: Band[]) {
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

function Bands({ plan, year, marks, onYear }: ChartsProps & { plan: string }) {
  const markets = useMonteCarlo(plan);
  if (markets.error) {
    return (
      <p className="text-muted-foreground py-8 text-sm">
        {markets.error.message}
      </p>
    );
  }
  if (!markets.data)
    return <Skeleton className="aspect-[4/3] sm:aspect-[5/2]" />;
  const [low, lower, median, upper, high] = PERCENTILES;
  const span = (from: number, to: number) =>
    `${percentileLabel(from)} – ${percentileLabel(to)}`;
  const config: ChartConfig = {
    outer: { label: span(low, high), color: SERIES[0] },
    inner: { label: span(lower, upper), color: SERIES[0] },
    median: { label: percentileLabel(median), color: SERIES[0] },
  };
  return (
    <Plot
      config={config}
      data={bandData(markets.data.bands)}
      label="Net worth through random markets"
      onYear={onYear}
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
      <Line
        isAnimationActive={false}
        dataKey="median"
        type="monotone"
        stroke="var(--color-median)"
        strokeWidth={2}
        dot={false}
      />
      {markLines({ year, marks })}
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
export function Charts(props: Omit<ChartsProps, "marks">) {
  const chartProps = { ...props, marks: props.series.marks };
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
              <Chart {...chartProps} />
            </TabsContent>
          ))}
        </CardContent>
      </Tabs>
    </Card>
  );
}
