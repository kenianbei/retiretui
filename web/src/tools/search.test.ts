import { describe, expect, it } from "vitest";

import { percentOf, toolSearch } from "@/tools/search";
import { heldIn, heldOf, keptSearch } from "@/year/search";

describe("toolSearch", () => {
  it("keeps a whole bracket, the nominal basis, and an open form", () => {
    expect(toolSearch({ bracket: "22", basis: "nominal", edit: true })).toEqual(
      { bracket: 22, basis: "nominal", edit: true },
    );
  });

  it("drops what does not read", () => {
    expect(toolSearch({ bracket: "high", basis: "today", edit: 1 })).toEqual(
      {},
    );
    expect(toolSearch({ claim: "70-", person: -1, held: "" })).toEqual({});
  });

  it("keeps the claims by their ages, a person, and who is held", () => {
    expect(
      toolSearch({ claim: "70-67", person: "1", held: "ann,bob" }),
    ).toEqual({ claim: "70-67", person: 1, held: "ann,bob" });
    expect(toolSearch({ claim: 70 })).toEqual({ claim: "70" });
  });

  it("keeps an order by its classes", () => {
    expect(toolSearch({ order: "taxable-roth-deferred" })).toEqual({
      order: "taxable-roth-deferred",
    });
    expect(toolSearch({ order: "Taxable, Roth" })).toEqual({});
  });

  it("keeps a spending ceiling by its key", () => {
    expect(toolSearch({ ceiling: "planned" })).toEqual({ ceiling: "planned" });
    expect(toolSearch({ ceiling: true })).toEqual({});
  });

  it("keeps the year, and the status and state whose tax tables show", () => {
    expect(toolSearch({ year: "2031", status: "single", state: "or" })).toEqual(
      { year: 2031, status: "single", state: "or" },
    );
    expect(toolSearch({ status: "", state: 5 })).toEqual({ state: "5" });
  });

  it("keeps a market run by its place or its start year", () => {
    expect(toolSearch({ run: "p10" })).toEqual({ run: "p10" });
    expect(toolSearch({ run: 1929 })).toEqual({ run: "1929" });
    expect(toolSearch({ run: true })).toEqual({});
  });

  it("reads a rate back as the percent it is kept under", () => {
    expect(percentOf(0.22)).toBe(22);
    expect(percentOf(0.1)).toBe(10);
  });
});

describe("keptSearch", () => {
  const held = { year: 2030, basis: "nominal", bracket: 22 };

  it("reads who is held and writes them back", () => {
    expect(heldOf({ held: "ann,bob" })).toEqual(["ann", "bob"]);
    expect(heldOf({})).toEqual([]);
    expect(heldIn(["ann"])).toBe("ann");
    expect(heldIn([])).toBeUndefined();
  });

  it("carries only the keys a route keeps", () => {
    expect(keptSearch(held, ["year", "basis"])).toEqual({
      year: 2030,
      basis: "nominal",
    });
    expect(keptSearch(held, ["basis"])).toEqual({ basis: "nominal" });
    expect(keptSearch(held, [])).toEqual({});
  });
});
