import { expect, it } from "vitest";

import { dollars, share } from "@/overview/words";

it("rounds money to whole dollars and shares to whole percents", () => {
  expect(dollars(1234567.89)).toBe("$1,234,568");
  expect(dollars(-50)).toBe("-$50");
  expect(share(0.873)).toBe("87%");
});
