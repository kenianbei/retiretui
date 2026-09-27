import { describe, expect, it } from "vitest";

import { actionsYear, dollars, share, step } from "@/overview/words";

const NAMES = {
  people: { sam: "Sam" },
  accounts: { "roth-ira-sam": "Sam's Roth IRA", "k401-sam": "Sam's 401(k)" },
};

describe("the overview's words", () => {
  it("rounds money to whole dollars and shares to whole percents", () => {
    expect(dollars(1234567.89)).toBe("$1,234,568");
    expect(dollars(-50)).toBe("-$50");
    expect(share(0.873)).toBe("87%");
  });

  it("shows this year's actions, held within the plan's years", () => {
    expect(actionsYear(2026, 2080, 2030)).toBe(2030);
    expect(actionsYear(2031, 2080, 2026)).toBe(2031);
    expect(actionsYear(2000, 2020, 2026)).toBe(2020);
  });

  it("names accounts by their display names, and by id where there is none", () => {
    const conversion = step(
      {
        kind: "conversion",
        from: "k401-sam",
        to: "roth-ira-sam",
        amount: 5000,
      },
      NAMES,
    );
    expect(conversion).toEqual({
      verb: "Convert",
      amount: 5000,
      detail: "Sam's 401(k) → Sam's Roth IRA",
    });
    const withdrawal = step(
      { kind: "withdrawal", account: "gone", amount: 1 },
      NAMES,
    );
    expect(withdrawal.detail).toBe("from gone");
  });

  it("counts an employer's share into a contribution, and says so", () => {
    const contribution = step(
      {
        kind: "contribution",
        account: "k401-sam",
        employee: 1000,
        employer: 500,
        notes: [],
      },
      NAMES,
    );
    expect(contribution.amount).toBe(1500);
    expect(contribution.detail).toBe(
      "to Sam's 401(k), $500 of it from the employer",
    );
  });
});
