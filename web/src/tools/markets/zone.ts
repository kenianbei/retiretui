import type { Zone } from "@wasm/retiretui_wasm.js";

/** The colour a share of runs reads in, by the zone it falls in. */
export const ZONE_CLASS: Record<Zone, string> = {
  good: "text-success",
  caution: "text-warning",
  short: "text-destructive",
};
