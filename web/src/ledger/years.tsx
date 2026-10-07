import type { Ledger, LedgerRow } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useRef } from "react";

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

/** The year, the ages and the marks, then every figure of the ledger's column set. */
function columnsOf(ledger: Ledger) {
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
    ...ledger.figure_headers.map((header, at) =>
      column.display({
        id: String(at),
        header,
        meta: { isNumeric: true },
        cell: ({ row }) => row.original.figures[at],
      }),
    ),
  ];
}

/**
 * Scrolls the table's own frame, and nothing the frame is in, until the year shown is clear of
 * its edges and of the headers that stay at its top: a reader down the page stepping the year
 * is not taken back up to the table.
 */
function useYearInView(year: number | undefined, isAlone: boolean) {
  const within = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const row = within.current?.querySelector('[aria-selected="true"]');
    const frame = row?.closest("table")?.parentElement;
    if (!row || !frame) return;
    const [edges, at] = [
      frame.getBoundingClientRect(),
      row.getBoundingClientRect(),
    ];
    const headers = frame.querySelector("thead")?.getBoundingClientRect();
    const top = edges.top + (headers?.height ?? 0);
    if (at.top < top) frame.scrollTop -= top - at.top;
    else if (at.bottom > edges.bottom)
      frame.scrollTop += at.bottom - edges.bottom;
  }, [year, isAlone]);
  return within;
}

/**
 * Every year in a table under the column set the ledger was asked for, the year shown kept in
 * view: over a year in full, in a frame short enough to leave the year in reach, or alone, in
 * one as tall as the screen.
 */
export function Years({
  ledger,
  unit,
  year,
  onSelect,
  isAlone,
}: {
  ledger: Ledger;
  unit: string;
  year: number | undefined;
  onSelect: (year: number) => void;
  isAlone: boolean;
}) {
  const columns = useMemo(() => columnsOf(ledger), [ledger]);
  const within = useYearInView(year, isAlone);
  return (
    <div ref={within} className="min-w-0">
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
        isFirstPinned
        className={isAlone ? "max-h-[calc(100dvh-12rem)]" : "max-h-[40dvh]"}
      />
    </div>
  );
}
