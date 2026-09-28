import { describe, expect, it } from "vitest";

import {
  CHECK,
  around,
  keepAnswers,
  keptAnswers,
  newPlanSearch,
  stepsShown,
} from "@/onboarding/steps";

const STEPS = [
  { slug: "household", title: "Household", keys: ["filing"] },
  { slug: "you", title: "You", keys: ["name"] },
  { slug: "partner", title: "Partner", keys: ["partner_name"] },
];

describe("the steps", () => {
  it("skip a step with nothing on show and end at the check", () => {
    const order = stepsShown(STEPS, new Set(["filing", "name"]));
    expect(order).toEqual(["household", "you", CHECK]);
    expect(around(order, "you")).toEqual({ before: "household", after: CHECK });
    expect(around(order, "household").before).toBeUndefined();
  });

  it("send a step no longer asked on to the first", () => {
    const order = stepsShown(STEPS, new Set(["filing"]));
    expect(around(order, "partner")).toEqual({ after: "household" });
  });

  it("read only what they hold from the address", () => {
    expect(
      newPlanSearch({ field: "name", isChanging: true, other: 1 }),
    ).toEqual({ field: "name", isChanging: true });
    expect(newPlanSearch({ isChanging: "yes" })).toEqual({});
  });
});

describe("the kept answers", () => {
  it("are read back until forgotten", () => {
    const storage = new Map<string, string>();
    const shim = {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    } as unknown as Storage;
    keepAnswers(shim, 'name = "Jordan"');
    expect(keptAnswers(shim)).toBe('name = "Jordan"');
    keepAnswers(shim, null);
    expect(keptAnswers(shim)).toBeNull();
  });
});
