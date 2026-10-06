import { describe, expect, it } from "vitest";

import { basisOf, ledgerSearch, yearSearch } from "@/year/search";

describe("yearSearch", () => {
  it("keeps a whole year and the nominal basis", () => {
    expect(yearSearch({ year: 2031, basis: "nominal" })).toEqual({
      year: 2031,
      basis: "nominal",
    });
    expect(yearSearch({ year: "2031" })).toEqual({ year: 2031 });
  });

  it("drops what is not a year or a basis", () => {
    expect(yearSearch({ year: 2031.5, basis: "today" })).toEqual({});
    expect(yearSearch({ year: "soon", basis: "real" })).toEqual({});
  });

  it("reads no basis as today's dollars", () => {
    expect(basisOf({})).toBe("today");
    expect(basisOf({ basis: "nominal" })).toBe("nominal");
  });
});

describe("ledgerSearch", () => {
  it("keeps the market a run went through beside the year", () => {
    expect(ledgerSearch({ year: 2040, market: "trial-423" })).toEqual({
      year: 2040,
      market: "trial-423",
    });
    expect(ledgerSearch({ market: 1929 })).toEqual({ market: "1929" });
  });

  it("keeps the table view and the column set it is under", () => {
    expect(ledgerSearch({ view: "table", columns: "tax" })).toEqual({
      view: "table",
      columns: "tax",
    });
    expect(ledgerSearch({ view: "year", columns: "" })).toEqual({});
    expect(ledgerSearch({ view: ["table"] })).toEqual({});
  });

  it("drops a market that is not text, or is none", () => {
    expect(ledgerSearch({ market: ["1929"] })).toEqual({});
    expect(ledgerSearch({ market: "" })).toEqual({});
  });
});
