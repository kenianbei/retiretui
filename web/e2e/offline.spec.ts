import type { Page } from "@playwright/test";

import { SEARCH, example, expect, seed, test } from "./support";

/** How many built files the page has cached. */
function cachedBuilt(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const cache = await caches.open("retiretui-app");
    const kept = await cache.keys();
    return kept.filter((request) => request.url.includes("/assets/")).length;
  });
}

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
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Overview", level: 1 }),
  ).toBeVisible();
  let last = -1;
  await expect
    .poll(
      async () => {
        const now = await cachedBuilt(page);
        const isSettled = now === last;
        last = now;
        return isSettled;
      },
      { intervals: [1_000] },
    )
    .toBe(true);

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
    page.getByText(/^money lasts in [\d.]+% of [\d,]+$/),
  ).toBeVisible(SEARCH);
  await context.setOffline(false);
});
