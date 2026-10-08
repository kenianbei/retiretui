import { Link } from "@tanstack/react-router";
import type { DetailLine, Funds, Year } from "@wasm/retiretui_wasm.js";

import { Titled } from "@/ledger/titled";
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

/** The year's tax: what was paid, then what it was worked out from, over a way to the tables. */
function TaxCard({ detail }: { detail: Year }) {
  return (
    <Titled title={VIEW_WORDS.tax}>
      <Lines label={VIEW_WORDS.paid} lines={detail.tax} />
      {detail.worked_from.length > 0 && (
        <Lines label={VIEW_WORDS.worked_from} lines={detail.worked_from} />
      )}
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

/** What the year lived on, where that went, and its tax. */
export function MoneyCards({ detail }: { detail: Year }) {
  return (
    <div className="grid grid-cols-1 items-start gap-4 @xl:grid-cols-2 @4xl:grid-cols-3">
      <FundsCard title={VIEW_WORDS.money_in} funds={detail.money_in} />
      <FundsCard title={VIEW_WORDS.money_out} funds={detail.money_out} />
      <TaxCard detail={detail} />
    </div>
  );
}
