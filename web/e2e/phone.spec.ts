import type { Page } from "@playwright/test";

import { example, expect, isPhone, searchesDone, seed, test } from "./support";

/** The page is no wider than the screen it is drawn on. */
async function expectFits(page: Page, hash: string) {
  const [scrolled, shown] = await page.evaluate(() => [
    document.documentElement.scrollWidth,
    document.documentElement.clientWidth,
  ]);
  expect(scrolled, `${hash} scrolls sideways`).toBeLessThanOrEqual(shown);
}

/** The pages a grouped tab's chips lead to, from one of them. */
async function pagesOf(
  page: Page,
  group: string,
  first: string,
): Promise<string[]> {
  await page.goto(first);
  const links = page.getByRole("navigation", { name: group }).getByRole("link");
  await expect(links.first()).toBeVisible();
  return links.evaluateAll((each) =>
    each.map((link) => link.getAttribute("href") ?? ""),
  );
}

const ONLY_PHONE = "the widths it holds to are a phone's";

test("every page fits a phone's width", async ({ page }, testInfo) => {
  test.skip(!isPhone(testInfo), ONLY_PHONE);
  test.setTimeout(300_000);
  await seed(
    page,
    { "/couple.toml": example("mid-career-couple.toml") },
    "/couple.toml",
  );
  const hashes = [
    "#/overview",
    "#/ledger",
    "#/compare",
    ...(await pagesOf(page, "Tools", "#/tools/roth-conversions")),
    ...(await pagesOf(page, "Plan", "#/plan/accounts")),
  ];
  expect(hashes.length, "the Tools and Plan pages were found").toBeGreaterThan(
    15,
  );
  for (const hash of hashes) {
    await page.goto(hash.startsWith("#") ? hash : `#${hash}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await searchesDone(page);
    await expectFits(page, hash);
  }
});
