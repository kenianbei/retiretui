import { Link } from "@tanstack/react-router";
import type {
  AssumptionRow,
  OverviewTotal,
  TotalLeads,
} from "@wasm/retiretui_wasm.js";
import { ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

import { pageOf, TOOLS } from "@/nav";
import { Rows } from "@/overview/lists";
import { ROW } from "@/overview/row";
import { BASIS_LABEL, VIEW_WORDS } from "@/overview/view-words";
import type { Basis } from "@/overview/words";
import { placeSearch } from "@/plan/search";
import { YearInLedger } from "@/year/ledger-link";
import { keptSearch } from "@/year/search";

/** Where a total's row leads, in words a reader is told. */
function destinationOf(leads: TotalLeads): string {
  if ("tool" in leads) return `in ${pageOf(TOOLS, leads.tool).title}`;
  return "year" in leads ? "in the Ledger" : "in the plan";
}

/** A total's row as a link to where it leads. */
function Leading({
  leads,
  children,
}: {
  leads: TotalLeads;
  children: ReactNode;
}) {
  if ("year" in leads) {
    return (
      <YearInLedger year={leads.year} className={ROW}>
        {children}
      </YearInLedger>
    );
  }
  if ("tool" in leads) {
    return (
      <Link
        to="/tools/$page"
        params={{ page: leads.tool }}
        search={(kept) => keptSearch(kept, ["basis", "held"])}
        className={ROW}
      >
        {children}
      </Link>
    );
  }
  const { domain, index } = leads.place;
  return (
    <Link
      to="/plan/$page"
      params={{ page: domain }}
      search={index === null ? {} : { item: index }}
      className={ROW}
    >
      {children}
    </Link>
  );
}

/** A total: what it sums beside the sum, over what it is made of. */
function TotalRow({ total }: { total: OverviewTotal }) {
  const said = (
    <span className="min-w-0 flex-1">
      <span className="flex items-baseline justify-between gap-3">
        <span>{total.label}</span>
        <span className="font-semibold tabular-nums">{total.amount}</span>
      </span>
      {total.made_of !== "" && (
        <span className="text-muted-foreground block text-xs">
          {total.made_of}
        </span>
      )}
    </span>
  );
  if (total.leads === null) {
    return <p className="flex min-h-11 px-4 py-2.5 text-sm">{said}</p>;
  }
  return (
    <Leading leads={total.leads}>
      {said}
      <span className="sr-only">, {destinationOf(total.leads)}</span>
      <ChevronRight aria-hidden className="text-muted-foreground size-4" />
    </Leading>
  );
}

/** Over the plan: what its years add up to, each total leading to its page or tool. */
export function OverThePlan({
  totals,
  basis,
}: {
  totals: readonly OverviewTotal[];
  basis: Basis;
}) {
  return (
    <section aria-labelledby="totals" className="min-w-0 space-y-3">
      <div className="space-y-0.5">
        <h2 id="totals" className="text-lg font-semibold">
          {VIEW_WORDS.over_the_plan}
        </h2>
        <p className="text-muted-foreground text-xs">{BASIS_LABEL[basis]}</p>
      </div>
      <Rows>
        {totals.map((total) => (
          <li key={total.label}>
            <TotalRow total={total} />
          </li>
        ))}
      </Rows>
    </section>
  );
}

/** Rests on: what the projection hangs from, each leading to the field it is edited at. */
export function RestsOn({ rows }: { rows: readonly AssumptionRow[] }) {
  return (
    <section aria-labelledby="rests-on" className="min-w-0 space-y-3">
      <h2 id="rests-on" className="text-lg font-semibold">
        {VIEW_WORDS.rests_on}
      </h2>
      <Rows>
        {rows.map((row) => {
          const place = { domain: row.domain, index: null, field: row.field };
          return (
            <li key={row.label}>
              <Link
                to="/plan/$page"
                params={{ page: row.domain }}
                search={row.field === null ? {} : placeSearch(place).search}
                className={ROW}
              >
                <span className="text-muted-foreground w-28 shrink-0">
                  {row.label}
                </span>
                <span className="min-w-0 flex-1">
                  {row.value}
                  <span className="sr-only">, in the plan</span>
                </span>
                <ChevronRight
                  aria-hidden
                  className="text-muted-foreground size-4"
                />
              </Link>
            </li>
          );
        })}
      </Rows>
    </section>
  );
}
