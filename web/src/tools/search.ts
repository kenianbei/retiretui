import { keptSearch, textOf, wholeOf, type YearSearch } from "@/year/search";

/** What a tool's page holds in its address. */
export interface ToolSearch extends YearSearch {
  /** The highlighted option's bracket, as a whole percent; the best where none. */
  bracket?: number;
  /** The highlighted claims, their ages joined: `70-67`; the best where none. */
  claim?: string;
  /** The highlighted person, by place in the household; the first where none. */
  person?: number;
  /** The highlighted market run: `planned`, `p90`, `worst`, a start year; the plan's own where none. */
  run?: string;
  /** Whether the constraints are open in their form. */
  edit?: true;
  /** The filing status whose tax tables are shown; the plan's where none. */
  status?: string;
  /** The state whose tax tables are shown; the plan's that year where none. */
  state?: string;
}

const CLAIM_KEY = /^\d+(-\d+)*$/;

/** A tool's search params from whatever the address holds. */
export function toolSearch(search: Record<string, unknown>): ToolSearch {
  const bracket = wholeOf(search.bracket);
  const person = wholeOf(search.person);
  const claim = textOf(search.claim);
  const run = textOf(search.run);
  const status = textOf(search.status);
  const state = textOf(search.state);
  return {
    ...keptSearch(search, ["year", "basis", "held"]),
    ...(bracket !== undefined && { bracket }),
    ...(claim !== undefined && CLAIM_KEY.test(claim) && { claim }),
    ...(person !== undefined && person >= 0 && { person }),
    ...(run !== undefined && { run }),
    ...((search.edit === true || search.edit === "true") && { edit: true }),
    ...(status && { status }),
    ...(state && { state }),
  };
}

/** A bracket's rate as the whole percent the address holds. */
export function percentOf(rate: number): number {
  return Math.round(rate * 100);
}
