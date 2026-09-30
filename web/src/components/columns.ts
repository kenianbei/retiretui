import {
  createColumnHelper,
  tableFeatures,
  type ColumnDef,
  type RowData,
} from "@tanstack/react-table";

/** What every table here knows of a column beyond its cells. */
export const FEATURES = tableFeatures({
  columnMeta: {} as {
    /** Figures line up on the right, in tabular numerals. */
    isNumeric: boolean;
    /** How the table is ordered by it, where it is. */
    sorted?: "ascending" | "descending";
    /**
     * Text longer than a cell: `wraps` over lines so the figures beside it
     * stay in view; `clipped` held to a name's width, whole in its title.
     */
    text?: "wraps" | "clipped";
  },
});

export function columnsFor<Row extends RowData>() {
  return createColumnHelper<typeof FEATURES, Row>();
}

export type TableColumns<Row extends RowData> = ColumnDef<
  typeof FEATURES,
  Row
>[];

const TEXT_FIT = {
  wraps: "min-w-48 whitespace-normal",
  clipped: "max-w-72 truncate",
};

/** How a column's cells hold their text: on one line unless it says. */
export function fitted(text: "wraps" | "clipped" | undefined): string {
  return text ? TEXT_FIT[text] : "whitespace-nowrap";
}

/** How a column's cells line up: a figure by its digits' places. */
export function aligned(isNumeric: boolean | undefined): string {
  return isNumeric ? "text-right tabular-nums" : "text-left";
}
