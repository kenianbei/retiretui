/** The column a table is ordered by for the view alone, and which way. */
export interface TableSort {
  column: number;
  isDescending: boolean;
}

/** What a press on `column`'s header makes of the order: up, down, then the plan's own. */
export function pressed(
  held: TableSort | null,
  column: number,
): TableSort | null {
  if (held?.column !== column) return { column, isDescending: false };
  return held.isDescending ? null : { column, isDescending: true };
}
