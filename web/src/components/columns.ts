import {
  type ColumnDef,
  createColumnHelper,
  type RowData,
  tableFeatures,
} from "@tanstack/react-table";

/** What every table here knows of a column beyond its cells. */
export const FEATURES = tableFeatures({
  columnMeta: {} as {
    /** Figures line up on the right, in tabular numerals. */
    isNumeric: boolean;
    /** How the table is ordered by it, where it is. */
    sorted?: "ascending" | "descending";
  },
});

export function columnsFor<Row extends RowData>() {
  return createColumnHelper<typeof FEATURES, Row>();
}

export type TableColumns<Row extends RowData> = ColumnDef<
  typeof FEATURES,
  Row
>[];

/** How a column's cells line up: a figure by its digits' places. */
export function aligned(isNumeric: boolean | undefined): string {
  return isNumeric ? "text-right tabular-nums" : "text-left";
}
