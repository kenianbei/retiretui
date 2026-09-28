/** How far into `title` the letters of `query` begin; `null` where they do not all appear, in order. */
function matchAt(title: string, query: string): number | null {
  const text = title.toLowerCase();
  const whole = text.indexOf(query);
  if (whole >= 0) return whole;
  let from = 0;
  let first: number | null = null;
  for (const letter of query.replace(/\s+/g, "")) {
    const at = text.indexOf(letter, from);
    if (at < 0) return null;
    first ??= at;
    from = at + 1;
  }
  return first === null ? null : SCATTERED + first;
}

/** Ranks a match of scattered letters after every match of the query whole. */
const SCATTERED = 1000;

/**
 * Those of `items` whose title holds `query`'s letters in order, the query
 * found whole first, then those it begins nearest the start of; ties keep
 * their order. Every item where the query is blank.
 */
export function ranked<T extends { title: string }>(
  items: readonly T[],
  query: string,
): T[] {
  const wanted = query.trim().toLowerCase();
  if (wanted === "") return [...items];
  return items
    .map((item) => ({ item, at: matchAt(item.title, wanted) }))
    .filter((found): found is { item: T; at: number } => found.at !== null)
    .sort((one, other) => one.at - other.at)
    .map(({ item }) => item);
}
