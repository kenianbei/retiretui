import { describe, expect, it } from "vitest";

import { gathered } from "@/lib/utils";

describe("gathered", () => {
  it("runs neighbours under one heading, and apart under none", () => {
    const runs = gathered([
      { key: "a", group: null },
      { key: "b", group: "Stocks" },
      { key: "c", group: "Stocks" },
      { key: "d", group: "Bonds" },
      { key: "e" },
    ]);
    expect(
      runs.map(({ group, items }) => [group, items.map((item) => item.key)]),
    ).toEqual([
      [null, ["a"]],
      ["Stocks", ["b", "c"]],
      ["Bonds", ["d"]],
      [null, ["e"]],
    ]);
  });
});
