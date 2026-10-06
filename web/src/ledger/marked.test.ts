import type { LedgerRow } from "@wasm/retiretui_wasm.js";
import { describe, expect, it } from "vitest";

import { markedBeside } from "@/ledger/marked";

function row(year: number, mark?: "milestone" | "warning"): LedgerRow {
  return {
    year,
    ages: "",
    figures: [],
    is_exceeded: false,
    marks: {
      is_milestone: mark === "milestone",
      has_warning: mark === "warning",
    },
  };
}

describe("markedBeside", () => {
  const rows = [
    row(2026, "milestone"),
    row(2027),
    row(2028, "warning"),
    row(2029),
    row(2030, "milestone"),
  ];

  it("finds the nearest marked year either side, never the year itself", () => {
    expect(markedBeside(rows, 2028)).toEqual([2026, 2030]);
    expect(markedBeside(rows, 2029)).toEqual([2028, 2030]);
  });

  it("has none past the plan's first and last marks", () => {
    expect(markedBeside(rows, 2026)).toEqual([undefined, 2028]);
    expect(markedBeside(rows, 2030)).toEqual([2028, undefined]);
    expect(markedBeside([row(2026), row(2027)], 2026)).toEqual([
      undefined,
      undefined,
    ]);
  });
});
