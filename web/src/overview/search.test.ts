import { expect, test } from "vitest";

import { chartOf, FIRST_CHART, overviewSearch } from "@/overview/search";

test("the address names the chart on show, the first where it names none it has", () => {
  expect(chartOf("markets")).toBe("markets");
  expect(chartOf(undefined)).toBe(FIRST_CHART);
  expect(chartOf("no-such-chart")).toBe(FIRST_CHART);
});

test("the Overview carries the year it holds none of, beside its chart", () => {
  expect(overviewSearch({ year: "2045", chart: "income", other: 1 })).toEqual({
    year: 2045,
    chart: "income",
  });
  expect(overviewSearch({ chart: 3 })).toEqual({});
});
