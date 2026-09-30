import { Link } from "@tanstack/react-router";
import {
  sortPressed,
  type DomainTable,
  type Sort,
  type TableRow,
} from "@wasm/retiretui_wasm.js";
import { ArrowDown, ArrowUp } from "lucide-react";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
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

/** A header that orders the table by its column: up, then down, then the plan's own. */
function SortHeader({
  header,
  at,
  sort,
  onSort,
}: {
  header: string;
  at: number;
  sort: Sort | null;
  onSort: (sort: Sort | null) => void;
}) {
  const isSorted = sort?.column === at;
  const Arrow = sort?.is_descending ? ArrowDown : ArrowUp;
  return (
    <button
      type="button"
      className="hover:text-foreground text-muted-foreground inline-flex items-center gap-1"
      onClick={() => {
        onSort(sortPressed(sort, at));
      }}
    >
      {header}
      {isSorted && <Arrow aria-hidden className="size-3" />}
    </button>
  );
}

const column = columnsFor<TableRow>();

/** Every column, sortable by its header, the highlighted row marked. */
function WideTable({ slug, table, sort, onSort, highlighted }: ItemTableProps) {
  const columns = useMemo(
    () =>
      table.columns.map((each, at) =>
        column.display({
          id: String(at),
          meta: {
            isNumeric: each.is_numeric,
            ...(sort?.column === at && {
              sorted: sort.is_descending ? "descending" : "ascending",
            }),
          },
          header: () => (
            <SortHeader
              header={each.header}
              at={at}
              sort={sort}
              onSort={onSort}
            />
          ),
          cell: ({ row }) => {
            const cell = row.original.cells[at];
            const text = cell?.text ?? "";
            return at === 0 ? (
              <RowLink
                slug={slug}
                row={row.original}
                className="font-medium underline-offset-4 hover:underline"
              >
                {text || row.original.name}
              </RowLink>
            ) : cell?.is_unstated ? (
              <span className="text-muted-foreground">{text}</span>
            ) : (
              text
            );
          },
        }),
      ),
    [table.columns, slug, sort, onSort],
  );
  return (
    <DataTable
      label={slug}
      columns={columns}
      rows={table.rows}
      rowKey={(row) => String(row.index)}
      isSelected={(row) => row.index === highlighted}
      className="hidden @lg:block"
    />
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
    <div className="space-y-3 @lg:hidden">
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

/** A domain's items: a table where it has the width, rows where it has not. */
export function ItemTable(props: ItemTableProps) {
  return (
    <>
      <WideTable {...props} />
      <NarrowRows {...props} />
    </>
  );
}
