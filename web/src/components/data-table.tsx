import { flexRender, useTable, type RowData } from "@tanstack/react-table";
import type { KeyboardEvent } from "react";

import { FEATURES, type TableColumns } from "@/components/columns";
import { cn } from "@/lib/utils";

interface DataTableProps<Row extends RowData> {
  /** What the table is, for a screen reader. */
  label: string;
  columns: TableColumns<Row>;
  rows: readonly Row[];
  rowKey: (row: Row) => string;
  isSelected?: (row: Row) => boolean;
  /** A click or ⏎ on a row; rows are not focusable without it. */
  onSelect?: (row: Row) => void;
  isExceeded?: (row: Row) => boolean;
  /** The first column stays in view while the rest scroll sideways. */
  isFirstPinned?: boolean;
  className?: string;
}

/**
 * A table whose rows are the plan's: figures in tabular numerals on the
 * right, the selected row marked, an exceeded one in the shortfall colour.
 */
export function DataTable<Row extends RowData>({
  label,
  columns,
  rows,
  rowKey,
  isSelected,
  onSelect,
  isExceeded,
  isFirstPinned = false,
  className,
}: DataTableProps<Row>) {
  const table = useTable<typeof FEATURES, Row>({
    features: FEATURES,
    columns,
    data: rows,
    getRowId: rowKey,
  });
  const isPinned = (at: number) => isFirstPinned && at === 0;
  const aligned = (isNumeric: boolean | undefined) =>
    isNumeric ? "text-right tabular-nums" : "text-left";
  const selecting = (row: Row) =>
    onSelect && {
      tabIndex: 0,
      onClick: () => {
        onSelect(row);
      },
      onKeyDown: (event: KeyboardEvent) => {
        if (event.key !== "Enter") return;
        event.preventDefault();
        onSelect(row);
      },
    };
  return (
    <div className={cn("bg-card overflow-auto rounded-md border", className)}>
      <table aria-label={label} className="w-full text-sm">
        <thead className="border-b">
          {table.getHeaderGroups().map((group) => (
            <tr key={group.id} className="bg-card">
              {group.headers.map((header, at) => (
                <th
                  key={header.id}
                  scope="col"
                  aria-sort={header.column.columnDef.meta?.sorted}
                  className={cn(
                    "bg-card sticky top-0 z-20 px-3 py-2 font-medium whitespace-nowrap",
                    aligned(header.column.columnDef.meta?.isNumeric),
                    isPinned(at) && "left-0 z-30",
                  )}
                >
                  {flexRender(
                    header.column.columnDef.header,
                    header.getContext(),
                  )}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody className="divide-y">
          {table.getRowModel().rows.map((row) => {
            const isRowSelected = isSelected?.(row.original) ?? false;
            return (
              <tr
                key={row.id}
                aria-selected={onSelect ? isRowSelected : undefined}
                {...selecting(row.original)}
                className={cn(
                  "bg-card hover:bg-muted",
                  onSelect && "cursor-pointer",
                  isRowSelected && "bg-accent hover:bg-accent",
                  isExceeded?.(row.original) && "text-destructive",
                )}
              >
                {row.getAllCells().map((cell, at) => (
                  <td
                    key={cell.id}
                    className={cn(
                      "px-3 py-2 whitespace-nowrap",
                      aligned(cell.column.columnDef.meta?.isNumeric),
                      isPinned(at) && "sticky left-0 z-10 bg-inherit",
                    )}
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
