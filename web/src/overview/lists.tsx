import { Link } from "@tanstack/react-router";
import type { OverviewRow } from "@wasm/retiretui_wasm.js";
import { ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { pageOf, TOOLS } from "@/nav";
import { YearInLedger } from "@/year/ledger-link";
import { keptSearch } from "@/year/search";

/** A list of rows that each lead somewhere, over hairlines. */
export function Rows({ children }: { children: ReactNode }) {
  return <ul className="bg-card divide-y rounded-md border">{children}</ul>;
}

const ROW =
  "hover:bg-muted flex min-h-11 items-center gap-3 px-4 py-2.5 text-sm focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50";

/** What a row says and where it leads, the destination named for a reader. */
function RowContent({
  year,
  text,
  destination,
  aside,
}: {
  year?: number | null;
  text: ReactNode;
  destination: string;
  aside?: ReactNode;
}) {
  return (
    <>
      {year != null && (
        <span className="text-muted-foreground w-10 shrink-0 tabular-nums">
          {year}
        </span>
      )}
      <span className="min-w-0 flex-1">
        {text}
        <span className="sr-only">, {destination}</span>
      </span>
      {aside}
      <ChevronRight aria-hidden className="text-muted-foreground size-4" />
    </>
  );
}

/** A row that opens a tool's page. */
export function ToolRow({
  page,
  onClick,
  children,
}: {
  page: string;
  onClick?: () => void;
  children: ReactNode;
}) {
  const { title } = pageOf(TOOLS, page);
  return (
    <li>
      <Link
        to="/tools/$page"
        params={{ page }}
        search={(kept) => keptSearch(kept, ["basis", "held"])}
        onClick={onClick}
        className={ROW}
      >
        <RowContent
          text={children}
          destination={`in ${title}`}
          aside={
            <span
              aria-hidden
              className="text-muted-foreground hidden text-xs sm:inline"
            >
              {title}
            </span>
          }
        />
      </Link>
    </li>
  );
}

/** A row of the client's: a dated one opens its year in the Ledger, any other its item. */
function PlanRow({ row }: { row: OverviewRow }) {
  if (row.year !== null) {
    const year = row.year;
    return (
      <YearInLedger year={year} className={ROW}>
        <RowContent year={year} text={row.text} destination="in the Ledger" />
      </YearInLedger>
    );
  }
  if (row.place === null) {
    return (
      <p className="flex min-h-11 items-center px-4 py-2.5 text-sm">
        {row.text}
      </p>
    );
  }
  const { domain, index } = row.place;
  return (
    <Link
      to="/plan/$page"
      params={{ page: domain }}
      search={index === null ? {} : { item: index }}
      className={ROW}
    >
      <RowContent text={row.text} destination="in the plan" />
    </Link>
  );
}

/** A titled list of the client's rows, or `empty` where it has none. */
export function RowList({
  id,
  title,
  rows,
  empty,
  className,
}: {
  id: string;
  title: string;
  rows: readonly OverviewRow[];
  empty?: string;
  className?: string;
}) {
  return (
    <section aria-labelledby={id} className={cn("space-y-3", className)}>
      <h2 id={id} className="text-lg font-semibold">
        {title}
      </h2>
      {rows.length === 0 ? (
        <p className="text-muted-foreground text-sm">{empty}</p>
      ) : (
        <Rows>
          {rows.map((row) => (
            <li key={`${String(row.year)} ${row.text}`}>
              <PlanRow row={row} />
            </li>
          ))}
        </Rows>
      )}
    </section>
  );
}
