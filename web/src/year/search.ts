import type { Basis } from "@/overview/words";

/** The year and the dollars the Overview and the Ledger show, in their address. */
export interface YearSearch {
  /** The year shown; today's, held within the plan, where none. */
  year?: number;
  /** Nominal dollars; today's where none. */
  basis?: "nominal";
}

/** The Overview's and the Ledger's search params from whatever the address holds. */
export function yearSearch(search: Record<string, unknown>): YearSearch {
  const year =
    typeof search.year === "string" ? Number(search.year) : search.year;
  return {
    ...(typeof year === "number" && Number.isInteger(year) && { year }),
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
