import { expect, it } from "vitest";

import { noteShown, shownIn } from "@/shell/shown";

const page = (slug: string) => ({ slug });
const TOOLS = { path: "/tools/$page", pages: ["roth", "claims"].map(page) };
const PLAN = { path: "/plan/$page", pages: ["accounts", "income"].map(page) };

it("a group's tab shows its first page until one of its pages has been shown", () => {
  expect(shownIn(TOOLS, "/ledger")?.slug).toBe("roth");
  expect(shownIn(PLAN, "/ledger")?.slug).toBe("accounts");
});

it("a group's tab stays on the page the address is at, noted or not", () => {
  expect(shownIn(TOOLS, "/tools/claims")?.slug).toBe("claims");
  expect(shownIn(TOOLS, "/ledger")?.slug).toBe("roth");
});

it("a group's tab comes back to the page last shown in it", () => {
  for (const pathname of ["/tools/claims", "/ledger", "/plan/nowhere"]) {
    noteShown(TOOLS, pathname);
    noteShown(PLAN, pathname);
  }
  expect(shownIn(TOOLS, "/ledger")?.slug).toBe("claims");
  expect(shownIn(PLAN, "/ledger")?.slug).toBe("accounts");
  noteShown(PLAN, "/plan/income");
  expect(shownIn(PLAN, "/tools/roth")?.slug).toBe("income");
  expect(shownIn(TOOLS, "/plan/income")?.slug).toBe("claims");
});

it("a page a group no longer holds is not come back to", () => {
  noteShown(PLAN, "/plan/income");
  const fewer = { ...PLAN, pages: [page("accounts")] };
  expect(shownIn(fewer, "/ledger")?.slug).toBe("accounts");
});
