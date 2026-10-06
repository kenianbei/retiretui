import type { Ledger, LedgerRow } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { VIEW_WORDS } from "@/overview/view-words";

const column = columnsFor<LedgerRow>();

/** What sets a year apart: a milestone, a warning, each said to a screen reader. */
function Marks({ row }: { row: LedgerRow }) {
  const { is_milestone: isMilestone, has_warning: hasWarning } = row.marks;
  return (
    <>
      {isMilestone && (
        <span className="text-primary">
          <span aria-hidden>◆</span>
          <span className="sr-only">milestone</span>
        </span>
      )}
      {hasWarning && (
        <span className="text-warning-foreground font-semibold">
          <span aria-hidden>!</span>
          <span className="sr-only">warning</span>
        </span>
      )}
    </>
  );
}

const MARKS = column.display({
  id: "marks",
  header: () => <span className="sr-only">Marks</span>,
  meta: { isNumeric: false },
  cell: ({ row }) => <Marks row={row.original} />,
});

/** The year and the ages, the marks, then the figures at `shown`, by their place among the ledger's columns. */
function columnsOf(ledger: Ledger, shown: readonly number[]) {
  const cell = (at: number) =>
    column.display({
      id: String(at),
      header: ledger.columns[at]?.header ?? "",
      meta: { isNumeric: ledger.columns[at]?.is_numeric ?? false },
      cell: ({ row }) => row.original.cells[at],
    });
  return [cell(0), cell(1), MARKS, ...shown.map(cell)];
}

interface YearsProps {
  ledger: Ledger;
  unit: string;
  year: number | undefined;
  onSelect: (year: number) => void;
}

/** Every year in a narrow list: the year, the ages, what marks it, and the net worth it ends on. */
export function YearsList({ ledger, unit, year, onSelect }: YearsProps) {
  const columns = useMemo(
    () => columnsOf(ledger, [ledger.columns.length - 1]),
    [ledger],
  );
  return (
    <DataTable
      label={`${VIEW_WORDS.years}, ${unit}`}
      columns={columns}
      rows={ledger.rows}
      rowKey={(row) => String(row.year)}
      isSelected={(row) => row.year === year}
      isExceeded={(row) => row.is_exceeded}
      onSelect={(row) => {
        onSelect(row.year);
      }}
      className="max-h-[calc(100dvh-10rem)]"
    />
  );
}

/** Every year in one table, under the column set the ledger was asked for. */
export function YearTable({ ledger, unit, year, onSelect }: YearsProps) {
  const columns = useMemo(() => {
    const figures = ledger.columns.map((_, at) => at).slice(2);
    return columnsOf(ledger, figures);
  }, [ledger]);
  return (
    <DataTable
      label={`The plan year by year, ${unit}`}
      columns={columns}
      rows={ledger.rows}
      rowKey={(row) => String(row.year)}
      isSelected={(row) => row.year === year}
      isExceeded={(row) => row.is_exceeded}
      onSelect={(row) => {
        onSelect(row.year);
      }}
      isFirstPinned
      className="max-h-[calc(100dvh-12rem)]"
    />
  );
}
