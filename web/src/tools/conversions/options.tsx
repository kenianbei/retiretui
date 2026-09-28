import {
  compactMoney,
  type LadderOption,
  type LaddersReply,
  type Summary,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { cn } from "@/lib/utils";
import type { Basis } from "@/overview/words";
import { ReadRows } from "@/plan/read-out";

/** A row of the options: the plan as it stands, or a bracket's ladder. */
interface OptionRow {
  label: string;
  summary: Summary;
  option: LadderOption | null;
}

/** The figures an option is chosen by, in the order the reply names them. */
function figuresOf(summary: Summary): number[] {
  return [
    summary.lifetime_unfunded,
    summary.final_net_worth,
    summary.lifetime_taxes,
    summary.lifetime_medicare,
  ];
}

function cellsOf(row: OptionRow): string[] {
  return [
    row.label,
    compactMoney(row.summary.lifetime_conversions),
    ...figuresOf(row.summary).map(compactMoney),
  ];
}

function rowsOf(found: LaddersReply, basis: Basis): OptionRow[] {
  const current: OptionRow = {
    label: found.current,
    summary: found.baseline[basis],
    option: null,
  };
  const options = found.brackets.map((option) => ({
    label: option.label,
    summary: option.figures[basis],
    option,
  }));
  return [current, ...options];
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
          cell: ({ row }) => cellsOf(row.original)[at],
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
        rowKey={(row) => row.label}
        isSelected={(row) => row.option !== null && row.option === highlighted}
        onSelect={(row) => {
          if (row.option) highlight(row.option);
        }}
        className="hidden md:block"
      />
      <div className="space-y-3 md:hidden">
        <ul className="bg-card divide-y rounded-md border">
          {rows.map((row) => {
            const isChosen = row.option !== null && row.option === highlighted;
            const cells = cellsOf(row);
            const content = (
              <>
                <span className="font-medium">{row.label}</span>
                <span className="tabular-nums">{cells[NARROW_FIGURE]}</span>
              </>
            );
            const place = "flex w-full justify-between gap-3 px-4 py-3";
            return (
              <li key={row.label}>
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
              .map((header, at) => [header, cellsOf(chosen)[at + 1] ?? ""])}
          />
        )}
      </div>
    </>
  );
}
