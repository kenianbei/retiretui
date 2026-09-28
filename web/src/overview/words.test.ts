import { expect, it } from "vitest";

import { dollars } from "@/overview/words";

it("rounds money to whole dollars", () => {
  expect(dollars(1234567.89)).toBe("$1,234,568");
  expect(dollars(-50)).toBe("-$50");
});
