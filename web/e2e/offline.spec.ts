import { readdirSync } from "node:fs";

import type { Page } from "@playwright/test";

import { example, expect, SEARCH, seed, test } from "./support";

/** How many built files the page has cached. */
function cachedBuilt(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const cache = await caches.open("retiretui-build");
    const kept = await cache.keys();
    return kept.filter((request) => request.url.includes("/assets/")).length;
  });
}

/** The files the build wrote, as the suite's server serves them. */
const BUILT = readdirSync(new URL("../dist/assets", import.meta.url));

test("after one visit every page opens offline, searches and all", async ({
  page,
  context,
  browserName,
}) => {
  test.skip(
    browserName === "webkit",
    "Playwright's WebKit fails a reload while offline with an internal error before the service worker can answer it",
  );
  await seed(
    page,
    { "/early.toml": example("early-retiree.toml") },
    "/early.toml",
  );
  await expect(
    page.getByRole("heading", { name: "Overview", level: 1 }),
  ).toBeVisible();
  await page.evaluate(() => navigator.serviceWorker.ready);
  const manifest = await page.evaluate(
    async () =>
      (await fetch("./manifest.webmanifest")).json() as Promise<{
        display: string;
      }>,
  );
  expect(manifest.display).toBe("standalone");
  expect(await cachedBuilt(page)).toBe(BUILT.length);

  await context.setOffline(true);
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Overview", level: 1 }),
  ).toBeVisible();
  for (const [hash, heading] of [
    ["#/ledger", "Ledger"],
    ["#/compare", "Compare"],
    ["#/tools/tax-tables", "Tax Tables"],
    ["#/plan/accounts", "Accounts"],
    ["#/tools/monte-carlo", "Monte Carlo"],
  ] as const) {
    await page.goto(hash);
    await expect(
      page.getByRole("heading", { name: heading, level: 1 }),
    ).toBeVisible();
  }
  await expect(
    page.getByText(/^Money lasts in [\d.]+% of [\d,]+ markets$/),
  ).toBeVisible(SEARCH);
  await context.setOffline(false);
});
