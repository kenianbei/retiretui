import type { LedgerRow } from "@wasm/retiretui_wasm.js";

/** The nearest years the client marked before and after `year`, as the table shows them. */
export function markedBeside(
  rows: readonly LedgerRow[],
  year: number,
): [number | undefined, number | undefined] {
  const marked = rows
    .filter(({ marks }) => marks.is_milestone || marks.has_warning)
    .map((row) => row.year);
  return [
    marked.filter((each) => each < year).at(-1),
    marked.find((each) => each > year),
  ];
}
