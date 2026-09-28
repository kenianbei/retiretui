import type { Basis } from "@/overview/words";

/** The year and the dollars the Overview and the Ledger show, in their address. */
export interface YearSearch {
  /** The year shown; today's, held within the plan, where none. */
  year?: number;
  /** Nominal dollars; today's where none. */
  basis?: "nominal";
  /** The people whose claims the claim search holds, by id, commas between. */
  held?: string;
}

/** A whole number the address holds, as text or as a number. */
export function wholeOf(value: unknown): number | undefined {
  const number = typeof value === "string" ? Number(value) : value;
  return typeof number === "number" && Number.isInteger(number)
    ? number
    : undefined;
}

/** Text the address holds, which reads a number-like value back as a number. */
export function textOf(value: unknown): string | undefined {
  if (typeof value === "number") return String(value);
  return typeof value === "string" ? value : undefined;
}

/** The Overview's and the Ledger's search params from whatever the address holds. */
export function yearSearch(search: Record<string, unknown>): YearSearch {
  const year = wholeOf(search.year);
  const held = typeof search.held === "string" ? search.held : "";
  return {
    ...(year !== undefined && { year }),
    ...(search.basis === "nominal" && { basis: "nominal" }),
    ...(held !== "" && { held }),
  };
}

/** What the Ledger holds in its address besides the year and the basis. */
export interface LedgerSearch extends YearSearch {
  /** The market the plan is shown in: `trial-423`, `1929`; its own where none. */
  market?: string;
}

/** The Ledger's search params from whatever the address holds. */
export function ledgerSearch(search: Record<string, unknown>): LedgerSearch {
  const market = textOf(search.market);
  return {
    ...yearSearch(search),
    ...(market !== undefined && { market }),
  };
}

/** A search key a tab's link carries to the route it opens. */
export type KeptKey = "year" | "basis" | "held";

/** Of what the address holds, only the keys `keeps` names. */
export function keptSearch(
  search: Record<string, unknown>,
  keeps: readonly KeptKey[],
): YearSearch {
  const { year, basis, held } = yearSearch(search);
  return {
    ...(keeps.includes("year") && year !== undefined && { year }),
    ...(keeps.includes("basis") && basis && { basis }),
    ...(keeps.includes("held") && held && { held }),
  };
}

/** The people whose claims are held, by id. */
export function heldOf(search: YearSearch): string[] {
  return search.held?.split(",").filter(Boolean) ?? [];
}

/** `held` as the address keeps it; none where no one is. */
export function heldIn(held: readonly string[]): string | undefined {
  return held.length === 0 ? undefined : held.join(",");
}

export function basisOf(search: YearSearch): Basis {
  return search.basis ?? "today";
}
