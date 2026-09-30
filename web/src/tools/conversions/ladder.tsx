import {
  money,
  type LadderOption,
  type LadderYear,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import type { Basis } from "@/overview/words";

const column = columnsFor<string[]>();

function cellsOf(year: LadderYear, basis: Basis): string[] {
  const isToday = basis === "today";
  return [
    String(year.year),
    year.from,
    money(isToday ? year.amount_today : year.amount),
    money(isToday ? year.taxable_today : year.taxable),
  ];
}

/** The highlighted ladder's conversions, year by year. */
export function Conversions({
  option,
  headers,
  basis,
  nothing,
}: {
  option: LadderOption;
  headers: string[];
  basis: Basis;
  /** What is said in place of a ladder that converts nothing. */
  nothing: string;
}) {
  const rows = useMemo(
    () => option.steps.map((year) => cellsOf(year, basis)),
    [option, basis],
  );
  const columns = useMemo(
    () =>
      headers.map((header, at) =>
        column.display({
          id: String(at),
          header,
          meta: { isNumeric: at >= 2 },
          cell: ({ row }) => row.original[at],
        }),
      ),
    [headers],
  );
  if (option.steps.length === 0) {
    return <p className="text-muted-foreground text-sm">{nothing}</p>;
  }
  return (
    <DataTable
      label={`${option.label} conversions`}
      columns={columns}
      rows={rows}
      rowKey={(cells) => cells.join("|")}
      className="max-h-[28rem]"
    />
  );
}
