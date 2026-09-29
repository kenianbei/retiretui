import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { cn } from "@/lib/utils";
import { ReadRows } from "@/plan/read-out";

/** A row of a search's options: the plan as it stands, or an option. */
export interface OptionRow<T> {
  /** What tells it from the others. */
  key: string;
  /** What it shows under each column. */
  cells: string[];
  /** What a phone's row names it by. */
  narrow: string;
  /** The option; `null` for the plan's own row. */
  option: T | null;
}

interface OptionsProps<T> {
  label: string;
  columns: readonly string[];
  rows: OptionRow<T>[];
  /** Which column a phone's row shows beside its name. */
  narrowFigure: number;
  /** How many of the leading columns hold words rather than figures. */
  words?: number;
  /** Abbreviated columns, each with its words in full. */
  spelledOut?: readonly (readonly [string, string])[];
  highlighted: T | undefined;
  highlight: (option: T) => void;
}

/**
 * A search's options under the plan's own row, the highlighted one marked:
 * a table, or on a phone rows of a name and one figure, the highlighted
 * one read out under its row.
 */
export function Options<T>({
  label,
  columns,
  rows,
  narrowFigure,
  words = 1,
  spelledOut = [],
  highlighted,
  highlight,
}: OptionsProps<T>) {
  const tableColumns = useMemo(() => {
    const column = columnsFor<OptionRow<T>>();
    const spelled = new Map(spelledOut);
    return columns.map((header, at) =>
      column.display({
        id: String(at),
        header: () => {
          const full = spelled.get(header);
          return full ? <abbr title={full}>{header}</abbr> : header;
        },
        meta: { isNumeric: at >= words },
        cell: ({ row }) => row.original.cells[at],
      }),
    );
  }, [columns, words, spelledOut]);
  return (
    <>
      <DataTable
        label={label}
        columns={tableColumns}
        rows={rows}
        rowKey={(row) => row.key}
        isSelected={(row) => row.option === highlighted}
        onSelect={(row) => {
          if (row.option) highlight(row.option);
        }}
        className="hidden max-h-[28rem] md:block"
      />
      <ul className="bg-card max-h-[28rem] divide-y overflow-auto rounded-md border text-sm md:hidden">
        {rows.map((row) => {
          const isChosen = row.option === highlighted;
          const content = (
            <>
              <span className="font-medium">{row.narrow}</span>
              <span className="tabular-nums">{row.cells[narrowFigure]}</span>
            </>
          );
          const place = "flex w-full justify-between gap-3 px-4 py-3";
          return (
            <li key={row.key}>
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
              {isChosen && (
                <ReadRows
                  rows={columns
                    .map((header, at) => ({
                      label: header,
                      text: row.cells[at] ?? "",
                      isFigure: at >= words,
                    }))
                    .slice(1)}
                  isFlush
                />
              )}
            </li>
          );
        })}
      </ul>
    </>
  );
}
