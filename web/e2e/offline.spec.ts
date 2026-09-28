import { example, expect, seed, test } from "./support";

/**
 * The pages loaded once idle rather than at first: the Ledger, Compare,
 * the tools, the Plan pages and the new-plan questions.
 */
const LAZY_PAGES = 5;

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
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const cache = await caches.open("retiretui-app");
        return (await cache.keys()).filter((kept) =>
          kept.url.includes("/assets/page-"),
        ).length;
      }),
    )
    .toBe(LAZY_PAGES);

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
  ).toBeVisible({ timeout: 110_000 });
  await context.setOffline(false);
});
