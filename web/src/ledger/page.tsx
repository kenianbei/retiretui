import { Link, useSearch } from "@tanstack/react-router";
import type { LedgerRow } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useRef } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { YearDetailCards } from "@/ledger/year-detail";
import { BASIS_LABEL } from "@/overview/words";
import { useSession } from "@/session";
import { basisOf } from "@/year/search";
import { useYear } from "@/year/use-year";
import { BasisSwitch, YearStepper } from "@/year/year";

/** Where the table scrolls within its own half of the screen, the detail under it. */
const WIDE = "(min-width: 1024px)";

const column = columnsFor<LedgerRow>();

/** The plan year by year, and the year shown's flows, income and tax. */
export function LedgerPage() {
  const { reading, issues } = useSession();
  const basis = basisOf(useSearch({ from: "/ledger" }));
  const isNominal = basis === "nominal";
  const shown = useYear();
  const { year, setYear } = shown;
  const ledger = useMemo(() => {
    try {
      return reading.document?.ledger(isNominal);
    } catch {
      return undefined;
    }
  }, [reading, isNominal]);
  const detail = useMemo(
    () =>
      ledger && year !== undefined
        ? reading.document?.yearDetail(year, isNominal)
        : undefined,
    [reading, ledger, year, isNominal],
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
  useEffect(() => {
    if (!window.matchMedia(WIDE).matches) return;
    table.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [year]);

  if (!reading.document) return null;
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
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <div className="flex flex-wrap items-center gap-2">
          <YearStepper shown={shown} />
          <BasisSwitch />
        </div>
      </div>
      {issues.length > 0 && (
        <p className="border-destructive text-muted-foreground border-l-4 px-3 text-sm">
          These are the last figures the plan had without issues.
        </p>
      )}
      <div ref={table}>
        <DataTable
          label={`The plan year by year, ${unit}`}
          columns={columns}
          rows={ledger.rows}
          rowKey={(row) => String(row.year)}
          isSelected={(row) => row.year === year}
          isExceeded={(row) => row.is_exceeded}
          onSelect={(row) => {
            setYear(row.year);
            if (!window.matchMedia(WIDE).matches) {
              details.current?.scrollIntoView({ block: "start" });
            }
          }}
          isFirstPinned
          className="lg:max-h-[50vh]"
        />
      </div>
      <div
        ref={details}
        className="grid scroll-mt-4 items-start gap-4 2xl:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]"
      >
        {detail && year !== undefined && (
          <YearDetailCards year={year} unit={unit} detail={detail} />
        )}
      </div>
    </div>
  );
}
