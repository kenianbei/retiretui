import {
  compactMoney,
  type LadderOption,
  type LadderYear,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import type { Basis } from "@/overview/words";

const column = columnsFor<LadderYear>();

function cellsOf(year: LadderYear, basis: Basis): string[] {
  const isToday = basis === "today";
  return [
    String(year.year),
    year.from,
    compactMoney(isToday ? year.amount_today : year.amount),
    compactMoney(isToday ? year.taxable_today : year.taxable),
  ];
}

/** The highlighted ladder's conversions, year by year. */
export function Conversions({
  option,
  headers,
  basis,
}: {
  option: LadderOption;
  headers: string[];
  basis: Basis;
}) {
  const columns = useMemo(
    () =>
      headers.map((header, at) =>
        column.display({
          id: String(at),
          header,
          meta: { isNumeric: at >= 2 },
          cell: ({ row }) => cellsOf(row.original, basis)[at],
        }),
      ),
    [headers, basis],
  );
  if (option.steps.length === 0) {
    return (
      <p className="text-muted-foreground text-sm">
        This ladder converts nothing under these constraints.
      </p>
    );
  }
  return (
    <DataTable
      label={`${option.label} conversions`}
      columns={columns}
      rows={option.steps}
      rowKey={(year) => `${year.source}-${String(year.year)}`}
    />
  );
}
