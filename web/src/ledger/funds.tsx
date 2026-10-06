import { Link } from "@tanstack/react-router";
import type {
  DetailLine,
  Funds,
  HistoryChart,
  Year,
} from "@wasm/retiretui_wasm.js";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { History } from "@/ledger/history";
import { cn } from "@/lib/utils";
import { VIEW_WORDS } from "@/overview/view-words";
import { keptSearch } from "@/year/search";

/** A label beside its amount, in what `className` sets it apart by. */
function Line({ line, className }: { line: DetailLine; className?: string }) {
  return (
    <div className={cn("flex justify-between gap-4 py-1.5", className)}>
      <dt>{line.label}</dt>
      <dd className="tabular-nums">{line.amount}</dd>
    </div>
  );
}

function Lines({ label, lines }: { label: string; lines: DetailLine[] }) {
  return (
    <dl aria-label={label} className="divide-y text-sm">
      {lines.map((line) => (
        <Line key={line.label} line={line} />
      ))}
    </dl>
  );
}

function Titled({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="px-4">
        <CardTitle>
          <h3>{title}</h3>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 px-4">{children}</CardContent>
    </Card>
  );
}

/** One side of the year's money: every amount in one list, what each kind comes to beneath it, then all of it. */
function FundsCard({ title, funds }: { title: string; funds: Funds }) {
  return (
    <Titled title={title}>
      <div role="group" aria-label={title} className="text-sm">
        <dl className="divide-y">
          {funds.lines.map((line) => (
            <Line key={line.label} line={line} />
          ))}
        </dl>
        <dl className="mt-3 border-t-2">
          {funds.sums.map((sum) => (
            <Line
              key={sum.label}
              line={sum}
              className="text-muted-foreground"
            />
          ))}
          <Line line={funds.total} className="font-medium" />
        </dl>
      </div>
    </Titled>
  );
}

/** The year's tax: what was paid, then the bracket reached and what it was worked out from, over a way to the tables. */
function TaxCard({ detail }: { detail: Year }) {
  const worked = [
    ...(detail.bracket ? [detail.bracket] : []),
    ...detail.picture,
  ];
  return (
    <Titled title={VIEW_WORDS.tax}>
      <Lines label="Paid" lines={detail.tax} />
      {worked.length > 0 && <Lines label="Worked out from" lines={worked} />}
      <Link
        to="/tools/$page"
        params={{ page: "tax-tables" }}
        search={(kept) => ({
          ...keptSearch(kept, ["basis", "held"]),
          year: detail.year,
        })}
        className="inline-block text-sm underline underline-offset-4"
      >
        Tax tables for {detail.year}
      </Link>
    </Titled>
  );
}

/** A topic's column: its card over its history, each in the row its neighbours' are in. */
function Topic({ children }: { children: React.ReactNode }) {
  return (
    <div className="row-span-2 grid grid-rows-subgrid gap-4">{children}</div>
  );
}

/** What the year lived on, where that went, and its tax, each over its history across the plan. */
export function MoneyCards({
  detail,
  histories,
  onYear,
}: {
  detail: Year;
  histories: readonly HistoryChart[];
  onYear: (year: number) => void;
}) {
  const cards = [
    <FundsCard key="in" title={VIEW_WORDS.money_in} funds={detail.money_in} />,
    <FundsCard
      key="out"
      title={VIEW_WORDS.money_out}
      funds={detail.money_out}
    />,
    <TaxCard key="tax" detail={detail} />,
  ];
  return (
    <div className="grid grid-cols-1 gap-4 @xl:grid-cols-2 @4xl:grid-cols-3">
      {cards.map((card, at) => {
        const history = histories[at];
        return (
          <Topic key={card.key}>
            {card}
            {history && (
              <History chart={history} year={detail.year} onYear={onYear} />
            )}
          </Topic>
        );
      })}
    </div>
  );
}
