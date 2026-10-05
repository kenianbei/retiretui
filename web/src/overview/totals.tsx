import { Link } from "@tanstack/react-router";
import type { AssumptionRow, OverviewTotal } from "@wasm/retiretui_wasm.js";

import { RowContent, Rows, ToolRow } from "@/overview/lists";
import { ROW } from "@/overview/row";
import { BASIS_LABEL, VIEW_WORDS } from "@/overview/view-words";
import type { Basis } from "@/overview/words";
import { placeSearch } from "@/plan/search";
import { YearInLedger } from "@/year/ledger-link";

/** A total: what it sums beside the sum, over what it is made of. */
function TotalSaid({ total }: { total: OverviewTotal }) {
  return (
    <>
      <span className="flex items-baseline justify-between gap-3">
        <span>{total.label}</span>
        <span className="font-semibold tabular-nums">{total.amount}</span>
      </span>
      {total.made_of !== "" && (
        <span className="text-muted-foreground block text-xs">
          {total.made_of}
        </span>
      )}
    </>
  );
}

/** A total's row, leading to its tool, its year in the Ledger or its page; nowhere where it leads none. */
function TotalRow({ total }: { total: OverviewTotal }) {
  const { leads } = total;
  const said = <TotalSaid total={total} />;
  if (leads === null) {
    return (
      <li className="px-4 py-2.5 text-sm">
        <span className="block">{said}</span>
      </li>
    );
  }
  if ("tool" in leads) return <ToolRow page={leads.tool}>{said}</ToolRow>;
  if ("year" in leads) {
    return (
      <li>
        <YearInLedger year={leads.year} className={ROW}>
          <RowContent text={said} destination="in the Ledger" />
        </YearInLedger>
      </li>
    );
  }
  const { domain, index } = leads.place;
  return (
    <li>
      <Link
        to="/plan/$page"
        params={{ page: domain }}
        search={index === null ? {} : { item: index }}
        className={ROW}
      >
        <RowContent text={said} destination="in the plan" />
      </Link>
    </li>
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
          <TotalRow key={total.label} total={total} />
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
                <RowContent text={row.value} destination="in the plan" />
              </Link>
            </li>
          );
        })}
      </Rows>
    </section>
  );
}
