import { textOf, type YearSearch, yearSearch } from "@/year/search";

/** The workspace files compared with the document, which every route keeps. */
export interface WithSearch {
  with?: string[];
}

/** The compared files from whatever the address holds, each once. */
export function withSearch(search: Record<string, unknown>): WithSearch {
  const listed = Array.isArray(search.with) ? search.with : [];
  const paths = [
    ...new Set(
      listed.filter((path): path is string => typeof path === "string"),
    ),
  ];
  return paths.length === 0 ? {} : { with: paths };
}

/** `paths` as the address keeps them; none where there are none. */
export function withIn(paths: readonly string[]): string[] | undefined {
  return paths.length === 0 ? undefined : [...paths];
}

/** What the Compare page holds in its address besides the year and the basis. */
export interface CompareSearch extends YearSearch {
  /** The highlighted plan's path; the document where none. */
  plan?: string;
  /** The path of the plan the others are measured against; the document where none. */
  baseline?: string;
  /** Each plan but the baseline read as its difference from it. */
  difference?: true;
  /** The metric charted and tabled year by year; net worth where none. */
  metric?: string;
}

/** The Compare page's search params from whatever the address holds. */
export function compareSearch(search: Record<string, unknown>): CompareSearch {
  const plan = textOf(search.plan);
  const baseline = textOf(search.baseline);
  const metric = textOf(search.metric);
  return {
    ...yearSearch(search),
    ...(plan && { plan }),
    ...(baseline && { baseline }),
    ...((search.difference === true || search.difference === "true") && {
      difference: true,
    }),
    ...(metric && { metric }),
  };
}

/**
 * `with` once `opened` takes the document's place: `opened` leaves it and
 * the document it replaces, at `left`, joins it where `opened` was.
 */
export function swapped(
  paths: readonly string[],
  opened: string,
  left: string | null,
): string[] {
  const joined = left === null || left === opened ? [] : [left];
  const at = paths.indexOf(opened);
  const kept = paths.filter((path) => path !== opened && path !== left);
  if (at === -1) return [...kept, ...joined];
  return [...kept.slice(0, at), ...joined, ...kept.slice(at)];
}
