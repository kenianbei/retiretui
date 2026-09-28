import { keptSearch, wholeOf, type YearSearch } from "@/year/search";

/** What a tool's page holds in its address. */
export interface ToolSearch extends Pick<YearSearch, "basis" | "held"> {
  /** The highlighted option's bracket, as a whole percent; the best where none. */
  bracket?: number;
  /** The highlighted claims, their ages joined: `70-67`; the best where none. */
  claim?: string;
  /** The highlighted person, by place in the household; the first where none. */
  person?: number;
  /** Whether the constraints are open in their form. */
  edit?: true;
}

const CLAIM_KEY = /^\d+(-\d+)*$/;

/** A tool's search params from whatever the address holds. */
export function toolSearch(search: Record<string, unknown>): ToolSearch {
  const bracket = wholeOf(search.bracket);
  const person = wholeOf(search.person);
  const claim =
    typeof search.claim === "number" ? String(search.claim) : search.claim;
  return {
    ...keptSearch(search, ["basis", "held"]),
    ...(bracket !== undefined && { bracket }),
    ...(typeof claim === "string" && CLAIM_KEY.test(claim) && { claim }),
    ...(person !== undefined && person >= 0 && { person }),
    ...((search.edit === true || search.edit === "true") && { edit: true }),
  };
}

/** A bracket's rate as the whole percent the address holds. */
export function percentOf(rate: number): number {
  return Math.round(rate * 100);
}
