import { describe, expect, it } from "vitest";

import { ranked } from "@/shell/match";

const titles = (items: { title: string }[]) => items.map(({ title }) => title);
const ITEMS = [
  { title: "Overview" },
  { title: "Tools · Roth Conversions" },
  { title: "Plan · Conversions" },
  { title: "Save as…" },
  { title: "Open couple.toml" },
];

describe("the palette's ranking", () => {
  it("keeps every item, in order, for a blank query", () => {
    expect(titles(ranked(ITEMS, "  "))).toEqual(titles(ITEMS));
  });

  it("puts the query found whole first, nearest the start first", () => {
    expect(titles(ranked(ITEMS, "conv"))).toEqual([
      "Plan · Conversions",
      "Tools · Roth Conversions",
    ]);
  });

  it("finds letters in order however they are spread, never out of order", () => {
    expect(titles(ranked(ITEMS, "ovw"))).toEqual(["Overview"]);
    expect(titles(ranked(ITEMS, "vsa"))).toEqual([]);
    expect(titles(ranked(ITEMS, "OPEN COUPLE"))).toEqual(["Open couple.toml"]);
  });
});
