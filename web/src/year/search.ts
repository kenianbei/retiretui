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

export function basisOf(search: YearSearch): Basis {
  return search.basis ?? "today";
}
