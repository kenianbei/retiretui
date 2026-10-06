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
        <span className="text-warning font-semibold">
          <span aria-hidden>!</span>
          <span className="sr-only">warning</span>
        </span>
      )}
    </>
  );
}

/** The year, the ages and the marks, then the figures at `shown`, by their place among the ledger's. */
function columnsOf(ledger: Ledger, shown: readonly number[]) {
  const [year, ages] = ledger.text_headers;
  return [
    column.display({
      id: "year",
      header: year,
      meta: { isNumeric: false },
      cell: ({ row }) => row.original.year,
    }),
    column.display({
      id: "ages",
      header: ages,
      meta: { isNumeric: false },
      cell: ({ row }) => row.original.ages,
    }),
    column.display({
      id: "marks",
      header: () => <span className="sr-only">Marks</span>,
      meta: { isNumeric: false },
      cell: ({ row }) => <Marks row={row.original} />,
    }),
    ...shown.map((at) =>
      column.display({
        id: String(at),
        header: ledger.figure_headers[at] ?? "",
        meta: { isNumeric: true },
        cell: ({ row }) => row.original.figures[at],
      }),
    ),
  ];
}

/**
 * Every year: as the whole table under the column set the ledger was asked
 * for, or as the narrow list beside a year, with the net worth it ends on.
 */
export function Years({
  ledger,
  unit,
  year,
  onSelect,
  isWhole,
}: {
  ledger: Ledger;
  unit: string;
  year: number | undefined;
  onSelect: (year: number) => void;
  isWhole: boolean;
}) {
  const columns = useMemo(() => {
    const figures = ledger.figure_headers.map((_, at) => at);
    return columnsOf(ledger, isWhole ? figures : figures.slice(-1));
  }, [ledger, isWhole]);
  const label = isWhole ? "The plan year by year" : VIEW_WORDS.years;
  return (
    <DataTable
      label={`${label}, ${unit}`}
      columns={columns}
      rows={ledger.rows}
      rowKey={(row) => String(row.year)}
      isSelected={(row) => row.year === year}
      isExceeded={(row) => row.is_exceeded}
      onSelect={(row) => {
        onSelect(row.year);
      }}
      isFirstPinned={isWhole}
      className={
        isWhole ? "max-h-[calc(100dvh-12rem)]" : "max-h-[calc(100dvh-10rem)]"
      }
    />
  );
}
