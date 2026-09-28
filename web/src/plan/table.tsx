import { Link } from "@tanstack/react-router";
import {
  sortPressed,
  type DomainTable,
  type Sort,
  type TableRow,
} from "@wasm/retiretui_wasm.js";
import { ArrowDown, ArrowUp } from "lucide-react";

import { INPUT, cn } from "@/lib/utils";

interface ItemTableProps {
  slug: string;
  table: DomainTable;
  sort: Sort | null;
  onSort: (sort: Sort | null) => void;
  /** The highlighted item's plan index. */
  highlighted: number | undefined;
}

/** A link that highlights the row's item, keeping the page's other params. */
function RowLink({
  slug,
  row,
  className,
  children,
}: {
  slug: string;
  row: TableRow;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <Link
      to="/plan/$page"
      params={{ page: slug }}
      search={{ item: row.index }}
      className={className}
    >
      {children}
    </Link>
  );
}

/** Every column, sortable by its header, the highlighted row marked. */
function WideTable({ slug, table, sort, onSort, highlighted }: ItemTableProps) {
  return (
    <div className="bg-card hidden overflow-x-auto rounded-md border md:block">
      <table className="w-full text-sm">
        <thead className="border-b">
          <tr>
            {table.columns.map((column, at) => {
              const isSorted = sort?.column === at;
              const Arrow = sort?.is_descending ? ArrowDown : ArrowUp;
              return (
                <th
                  key={column.header}
                  scope="col"
                  aria-sort={
                    isSorted
                      ? sort.is_descending
                        ? "descending"
                        : "ascending"
                      : undefined
                  }
                  className={cn(
                    "px-3 py-2 font-medium",
                    column.is_numeric ? "text-right" : "text-left",
                  )}
                >
                  <button
                    type="button"
                    className="hover:text-foreground text-muted-foreground inline-flex items-center gap-1"
                    onClick={() => {
                      onSort(sortPressed(sort, at));
                    }}
                  >
                    {column.header}
                    {isSorted && <Arrow aria-hidden className="size-3" />}
                  </button>
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody className="divide-y">
          {table.rows.map((row) => (
            <tr
              key={row.index}
              className={cn(
                "hover:bg-accent/50",
                row.index === highlighted && "bg-accent",
              )}
            >
              {row.cells.map((cell, at) => (
                <td
                  key={at}
                  className={cn(
                    "px-3 py-2",
                    table.columns[at]?.is_numeric && "text-right tabular-nums",
                  )}
                >
                  {at === 0 ? (
                    <RowLink
                      slug={slug}
                      row={row}
                      className="font-medium underline-offset-4 hover:underline"
                    >
                      {cell.text || row.name}
                    </RowLink>
                  ) : (
                    cell.text
                  )}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** A phone's rows: each item's name and one figure over its kind. */
function NarrowRows({ slug, table, sort, onSort }: ItemTableProps) {
  const figure = table.columns.findIndex((column) => column.is_numeric);
  const kind = table.columns.findIndex(
    (column, at) => at > 0 && !column.is_numeric,
  );
  const sortValue =
    sort === null
      ? ""
      : `${String(sort.column)}:${sort.is_descending ? "down" : "up"}`;
  return (
    <div className="space-y-3 md:hidden">
      <label className="flex items-center gap-2 text-sm">
        <span className="text-muted-foreground">Sort by</span>
        <select
          className={cn(INPUT, "flex-1")}
          value={sortValue}
          onChange={(event) => {
            const [column, way] = event.target.value.split(":");
            onSort(
              column === undefined || column === ""
                ? null
                : { column: Number(column), is_descending: way === "down" },
            );
          }}
        >
          <option value="">The plan's order</option>
          {table.columns.flatMap((column, at) => [
            <option key={`${String(at)}:up`} value={`${String(at)}:up`}>
              {column.header}, lowest first
            </option>,
            <option key={`${String(at)}:down`} value={`${String(at)}:down`}>
              {column.header}, highest first
            </option>,
          ])}
        </select>
      </label>
      <ul className="bg-card divide-y rounded-md border">
        {table.rows.map((row) => (
          <li key={row.index}>
            <RowLink
              slug={slug}
              row={row}
              className="flex items-baseline justify-between gap-3 px-4 py-3"
            >
              <span className="min-w-0">
                <span className="block truncate font-medium">{row.name}</span>
                {kind >= 0 && (
                  <span className="text-muted-foreground block truncate text-sm">
                    {row.cells[kind]?.text}
                  </span>
                )}
              </span>
              {figure >= 0 && (
                <span className="tabular-nums">{row.cells[figure]?.text}</span>
              )}
            </RowLink>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** A domain's items: a table on a wide screen, rows on a phone. */
export function ItemTable(props: ItemTableProps) {
  return (
    <>
      <WideTable {...props} />
      <NarrowRows {...props} />
    </>
  );
}
