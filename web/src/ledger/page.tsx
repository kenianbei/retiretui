import { Link, useSearch } from "@tanstack/react-router";
import type { LedgerRow } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useRef, useSyncExternalStore } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { YearDetailCards } from "@/ledger/year-detail";
import { messageOf } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { basisOf, type LedgerSearch } from "@/year/search";
import { useYear } from "@/year/use-year";
import { BasisSwitch, YearStepper } from "@/year/year";

/** Where the table scrolls within its own half of the screen, the detail under it. */
const WIDE = "(min-width: 1024px)";

let wideQuery: MediaQueryList | undefined;

function wide(): MediaQueryList {
  wideQuery ??= window.matchMedia(WIDE);
  return wideQuery;
}

function followWide(changed: () => void) {
  wide().addEventListener("change", changed);
  return () => {
    wide().removeEventListener("change", changed);
  };
}

/** Whether the table leads, the year's detail under it; narrower, the detail leads. */
function useIsWide(): boolean {
  return useSyncExternalStore(followWide, () => wide().matches);
}

const column = columnsFor<LedgerRow>();

/** The Ledger in the plan's own market, the year and basis kept. */
function BackToPlan() {
  return (
    <Link
      to="/ledger"
      search={(kept) => ({ ...kept, market: undefined })}
      className="underline underline-offset-4"
    >
      Back to the plan
    </Link>
  );
}

/** The plan year by year, and the year shown's flows, income and tax. */
export function LedgerPage() {
  const { reading, issues } = useSession();
  const search: LedgerSearch = useSearch({ from: "/ledger" });
  const { market } = search;
  const basis = basisOf(search);
  const isNominal = basis === "nominal";
  const shown = useYear();
  const { year, setYear } = shown;
  const replayed = useMemo(() => {
    const document = reading.document;
    try {
      return {
        ledger: document?.ledger(isNominal, market),
        said: market === undefined ? "" : document?.marketSaid(market),
      };
    } catch (thrown) {
      return { refusal: messageOf(thrown) };
    }
  }, [reading, isNominal, market]);
  const ledger = replayed.ledger;
  const detail = useMemo(
    () =>
      ledger && year !== undefined
        ? reading.document?.yearDetail(year, isNominal, market)
        : undefined,
    [reading, ledger, year, isNominal, market],
  );
  const columns = useMemo(
    () =>
      (ledger?.columns ?? []).map((each, at) =>
        column.display({
          id: String(at),
          header: each.header,
          meta: { isNumeric: each.is_numeric },
          cell: ({ row }) => row.original.cells[at],
        }),
      ),
    [ledger],
  );
  const table = useRef<HTMLDivElement>(null);
  const details = useRef<HTMLDivElement>(null);
  const isWide = useIsWide();
  useEffect(() => {
    if (!isWide) return;
    table.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [year, isWide]);

  if (!reading.document) return null;
  if (market !== undefined && "refusal" in replayed) {
    return (
      <div className="max-w-3xl space-y-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <p className="text-muted-foreground">
          {replayed.refusal}. <BackToPlan />
        </p>
      </div>
    );
  }
  if (!ledger) {
    return (
      <div className="max-w-3xl space-y-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <p className="text-muted-foreground">
          The ledger shows once the plan&apos;s issues are fixed; the{" "}
          <Link to="/overview" className="underline underline-offset-4">
            Overview
          </Link>{" "}
          lists them.
        </p>
      </div>
    );
  }
  const unit = BASIS_LABEL[basis];
  const years = (
    <div key="years" ref={table}>
      <DataTable
        label={`The plan year by year, ${unit}`}
        columns={columns}
        rows={ledger.rows}
        rowKey={(row) => String(row.year)}
        isSelected={(row) => row.year === year}
        isExceeded={(row) => row.is_exceeded}
        onSelect={(row) => {
          setYear(row.year);
          if (!isWide) {
            details.current?.scrollIntoView({ block: "start" });
          }
        }}
        isFirstPinned
        className="max-h-[70dvh] lg:max-h-[50vh]"
      />
    </div>
  );
  const yearDetail = (
    <div
      key="detail"
      ref={details}
      className="grid min-w-0 scroll-mt-4 grid-cols-1 items-start gap-4 2xl:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]"
    >
      {detail && year !== undefined && (
        <YearDetailCards year={year} unit={unit} detail={detail} />
      )}
    </div>
  );
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <div className="flex flex-wrap items-center gap-2">
          <YearStepper shown={shown} />
          <BasisSwitch />
        </div>
      </div>
      {market !== undefined && (
        <p className="border-primary bg-card rounded-md border-l-4 px-3 py-2 text-sm first-letter:uppercase">
          {replayed.said}: the plan as a market tool ran it. <BackToPlan />
        </p>
      )}
      {issues.length > 0 && (
        <p className="border-destructive text-muted-foreground border-l-4 px-3 text-sm">
          These are the last figures the plan had without issues.
        </p>
      )}
      {isWide ? [years, yearDetail] : [yearDetail, years]}
    </div>
  );
}
