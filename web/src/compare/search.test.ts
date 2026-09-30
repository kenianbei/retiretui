import { describe, expect, it } from "vitest";

import { compareSearch, swapped, withSearch } from "@/compare/search";

describe("withSearch", () => {
  it("keeps each compared path once", () => {
    expect(withSearch({ with: ["/a.toml", "/b.toml", "/a.toml"] })).toEqual({
      with: ["/a.toml", "/b.toml"],
    });
  });

  it("drops what is not a list of paths", () => {
    expect(withSearch({ with: "/a.toml" })).toEqual({});
    expect(withSearch({ with: [] })).toEqual({});
    expect(withSearch({ with: [3] })).toEqual({});
  });
});

describe("compareSearch", () => {
  it("reads a number-like name back as text", () => {
    expect(compareSearch({ plan: 2031, baseline: "/b.toml" })).toEqual({
      plan: "2031",
      baseline: "/b.toml",
    });
  });

  it("keeps the difference and the metric", () => {
    expect(
      compareSearch({ difference: true, metric: "taxes", year: 2031 }),
    ).toEqual({ difference: true, metric: "taxes", year: 2031 });
    expect(compareSearch({ difference: false })).toEqual({});
  });
});

describe("swapped", () => {
  it("puts the document left where the opened plan was", () => {
    expect(
      swapped(["/a.toml", "/b.toml", "/c.toml"], "/b.toml", "/plan.toml"),
    ).toEqual(["/a.toml", "/plan.toml", "/c.toml"]);
  });

  it("adds the document left after a plan not compared", () => {
    expect(swapped(["/a.toml"], "/new.toml", "/plan.toml")).toEqual([
      "/a.toml",
      "/plan.toml",
    ]);
    expect(swapped(["/a.toml"], "/a.toml", null)).toEqual([]);
  });
});
