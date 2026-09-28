import { describe, expect, it } from "vitest";

import { basisOf, yearSearch } from "@/year/search";

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
