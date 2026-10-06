import { VIEW_WORDS } from "@/overview/view-words";

/** A column set as the address names it and as a heading says it. */
type ColumnSet = (typeof VIEW_WORDS.column_sets)[number];

/** The set the year table opens under, which the address does not keep. */
export const [FIRST_SET] = VIEW_WORDS.column_sets;

/** The column set the address names, as the client names it; the first where it names none. */
export function columnSetOf(named: string | undefined): string | undefined {
  const sets: readonly ColumnSet[] = VIEW_WORDS.column_sets;
  return (sets.find(([slug]) => slug === named) ?? FIRST_SET)?.[0];
}
