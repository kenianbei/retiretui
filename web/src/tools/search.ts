import { keptSearch, wholeOf, type YearSearch } from "@/year/search";

/** What a tool's page holds in its address. */
export interface ToolSearch extends Pick<YearSearch, "basis"> {
  /** The highlighted option's bracket, as a whole percent; the best where none. */
  bracket?: number;
  /** Whether the constraints are open in their form. */
  edit?: true;
}

/** A tool's search params from whatever the address holds. */
export function toolSearch(search: Record<string, unknown>): ToolSearch {
  const bracket = wholeOf(search.bracket);
  return {
    ...keptSearch(search, ["basis"]),
    ...(bracket !== undefined && { bracket }),
    ...((search.edit === true || search.edit === "true") && { edit: true }),
  };
}

/** A bracket's rate as the whole percent the address holds. */
export function percentOf(rate: number): number {
  return Math.round(rate * 100);
}
