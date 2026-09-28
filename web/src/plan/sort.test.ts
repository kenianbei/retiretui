import { expect, it } from "vitest";

import { pressed } from "@/plan/sort";

it("goes up, then down, then back to the plan's order", () => {
  const up = pressed(null, 2);
  expect(up).toEqual({ column: 2, isDescending: false });
  const down = pressed(up, 2);
  expect(down).toEqual({ column: 2, isDescending: true });
  expect(pressed(down, 2)).toBeNull();
  expect(pressed(down, 0)).toEqual({ column: 0, isDescending: false });
});
