import { readFileSync } from "node:fs";

import { expect, it } from "vitest";

import { ISSUES_URL } from "@/links";

it("index.html, which can import nothing, reports where the app does", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  expect(html).toContain(`href="${ISSUES_URL}"`);
});
