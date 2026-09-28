import {
  compactMoney,
  type LadderOption,
  type LaddersReply,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { cn } from "@/lib/utils";
import type { Basis } from "@/overview/words";
import { ReadRows } from "@/plan/read-out";

/** A row of the options: the plan as it stands, or a bracket's ladder. */
interface OptionRow {
  cells: string[];
  option: LadderOption | null;
}

function rowsOf(found: LaddersReply, basis: Basis): OptionRow[] {
  const row = (label: string, amounts: number[]) => [
    label,
    ...amounts.map(compactMoney),
  ];
  return [
    { cells: row(found.current, found.baseline[basis]), option: null },
    ...found.brackets.map((option) => ({
      cells: row(option.label, option.figures[basis]),
      option,
    })),
  ];
}

const column = columnsFor<OptionRow>();
/** Which of the columns a phone's row shows beside the bracket: final net. */
const NARROW_FIGURE = 3;

interface OptionsProps {
  found: LaddersReply;
  basis: Basis;
  highlighted: LadderOption | undefined;
  highlight: (option: LadderOption) => void;
}

/** Every bracket's ladder under the plan's own row, the highlighted one marked. */
export function Options({
  found,
  basis,
  highlighted,
  highlight,
}: OptionsProps) {
  const rows = useMemo(() => rowsOf(found, basis), [found, basis]);
  const columns = useMemo(
    () =>
      found.columns.map((header, at) =>
        column.display({
          id: String(at),
          header,
          meta: { isNumeric: at > 0 },
          cell: ({ row }) => row.original.cells[at],
        }),
      ),
    [found.columns],
  );
  const chosen = rows.find((row) => row.option === highlighted);
  return (
    <>
      <DataTable
        label="Ladder options"
        columns={columns}
        rows={rows}
        rowKey={(row) => row.cells[0] ?? ""}
        isSelected={(row) => row.option === highlighted}
        onSelect={(row) => {
          if (row.option) highlight(row.option);
        }}
        className="hidden md:block"
      />
      <div className="space-y-3 md:hidden">
        <ul className="bg-card divide-y rounded-md border">
          {rows.map((row) => {
            const isChosen = row.option === highlighted;
            const content = (
              <>
                <span className="font-medium">{row.cells[0]}</span>
                <span className="tabular-nums">{row.cells[NARROW_FIGURE]}</span>
              </>
            );
            const place = "flex w-full justify-between gap-3 px-4 py-3";
            return (
              <li key={row.cells[0]}>
                {row.option ? (
                  <button
                    type="button"
                    aria-pressed={isChosen}
                    className={cn(place, isChosen && "bg-accent")}
                    onClick={() => {
                      if (row.option) highlight(row.option);
                    }}
                  >
                    {content}
                  </button>
                ) : (
                  <div className={cn(place, "text-muted-foreground")}>
                    {content}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
        {chosen && (
          <ReadRows
            rows={found.columns
              .slice(1)
              .map((header, at) => [header, chosen.cells[at + 1] ?? ""])}
          />
        )}
      </div>
    </>
  );
}
