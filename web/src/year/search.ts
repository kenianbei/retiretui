import type { Basis } from "@/overview/words";

/** The year and the dollars the Overview and the Ledger show, in their address. */
export interface YearSearch {
  /** The year shown; today's, held within the plan, where none. */
  year?: number;
  /** Nominal dollars; today's where none. */
  basis?: "nominal";
}

/** A whole number the address holds, as text or as a number. */
export function wholeOf(value: unknown): number | undefined {
  const number = typeof value === "string" ? Number(value) : value;
  return typeof number === "number" && Number.isInteger(number)
    ? number
    : undefined;
}

/** The Overview's and the Ledger's search params from whatever the address holds. */
export function yearSearch(search: Record<string, unknown>): YearSearch {
  const year = wholeOf(search.year);
  return {
    ...(year !== undefined && { year }),
    ...(search.basis === "nominal" && { basis: "nominal" }),
  };
}

/** A search key a tab's link carries to the route it opens. */
export type KeptKey = "year" | "basis";

/** Of what the address holds, only the keys `keeps` names. */
export function keptSearch(
  search: Record<string, unknown>,
  keeps: readonly KeptKey[],
): YearSearch {
  const { year, basis } = yearSearch(search);
  return {
    ...(keeps.includes("year") && year !== undefined && { year }),
    ...(keeps.includes("basis") && basis && { basis }),
  };
}

export function basisOf(search: YearSearch): Basis {
  return search.basis ?? "today";
}
