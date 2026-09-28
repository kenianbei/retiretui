import { describe, expect, it } from "vitest";

import { percentOf, toolSearch } from "@/tools/search";
import { keptSearch } from "@/year/search";

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
  });

  it("reads a rate back as the percent it is kept under", () => {
    expect(percentOf(0.22)).toBe(22);
    expect(percentOf(0.1)).toBe(10);
  });
});

describe("keptSearch", () => {
  const held = { year: 2030, basis: "nominal", bracket: 22 };

  it("carries only the keys a route keeps", () => {
    expect(keptSearch(held, ["year", "basis"])).toEqual({
      year: 2030,
      basis: "nominal",
    });
    expect(keptSearch(held, ["basis"])).toEqual({ basis: "nominal" });
    expect(keptSearch(held, [])).toEqual({});
  });
});
