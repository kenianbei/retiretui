import { describe, expect, it } from "vitest";

import {
  CHECK,
  around,
  keepAnswers,
  keptAnswers,
  newPlanSearch,
  stepsAsked,
} from "@/onboarding/steps";

const STEPS = [
  { slug: "household", title: "Household", keys: ["filing"] },
  { slug: "you", title: "You", keys: ["name"] },
  { slug: "partner", title: "Partner", keys: ["partner_name"] },
];

describe("the steps", () => {
  it("skip a step with nothing on show and end at the check", () => {
    const asked = stepsAsked(STEPS, new Set(["filing", "name"]));
    expect(asked.map((step) => step.slug)).toEqual(["household", "you"]);
    expect(around(asked, "you")).toMatchObject({
      isAsked: true,
      place: 2,
      count: 3,
      before: "household",
      after: CHECK,
    });
    expect(around(asked, "household").before).toBeUndefined();
    expect(around(asked, CHECK).after).toBeUndefined();
    expect(around(asked, "partner").isAsked).toBe(false);
  });

  it("read only a field from the address", () => {
    expect(newPlanSearch({ field: "name", other: 1 })).toEqual({
      field: "name",
    });
    expect(newPlanSearch({ field: 3 })).toEqual({});
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
