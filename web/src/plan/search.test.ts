import { expect, it } from "vitest";

import { placeSearch, planSearch } from "@/plan/search";

it("keeps only the search params a domain page can use", () => {
  expect(planSearch({ item: "2", edit: "new", field: "name" })).toEqual({
    item: 2,
    edit: "new",
    field: "name",
  });
  expect(planSearch({ item: "-1", edit: "later", field: 3 })).toEqual({});
  expect(planSearch({ edit: 0 })).toEqual({ edit: 0 });
});

it("lands an issue on its item's form at its field", () => {
  const listed = { domain: "accounts", index: 1, field: "balance" };
  expect(placeSearch(listed)).toEqual({
    page: "accounts",
    search: { item: 1, edit: 1, field: "balance" },
  });
  const single = { domain: "settings", index: null, field: null };
  expect(placeSearch(single)).toEqual({
    page: "settings",
    search: { edit: 0 },
  });
});
