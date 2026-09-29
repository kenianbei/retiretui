import {
  SEARCH,
  example,
  expect,
  expectAccessible,
  isPhone,
  rowNamed,
  seed,
  test,
} from "./support";

const ZONED = /text-(success|warning|destructive)/;

test("the plan through random markets and history, a run opened in the Ledger", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  await seed(page, { "/mix.toml": example("market-mix.toml") }, "/mix.toml");
  const row = (name: string) => rowNamed(page, testInfo, name);

  const figure = page.getByText(/^\d+(\.\d+)?% of [\d,]+$/).first();
  await expect(figure).toHaveClass(ZONED, SEARCH);

  await page.goto("#/tools/monte-carlo");
  await expect(
    page.getByRole("heading", { name: "Monte Carlo" }),
  ).toBeVisible();
  const verdict = page.getByText(/^money lasts in [\d.]+% of [\d,]+$/);
  await expect(verdict).toHaveClass(ZONED, SEARCH);
  await expectAccessible(page);
  await row("10th percentile").click();
  await page.waitForURL(/run=p10/);
  await page.reload();
  await expect(verdict).toBeVisible(SEARCH);
  await expect(row("10th percentile")).toHaveAttribute(
    phone ? "aria-pressed" : "aria-selected",
    "true",
  );
  await expect(row("50th percentile")).toContainText(/\$\d{1,3}(,\d{3})+/);

  await page.getByRole("tab", { name: "By year" }).click();
  await expect(
    page.getByRole("columnheader", { name: "Funded" }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Funded" }).click();
  await expect(
    page.getByLabel("Share of runs still funded").locator("svg").first(),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Endings" }).click();
  await expect(
    page.getByLabel("What the runs end with").getByText("short").first(),
  ).toBeVisible();
  await expect(
    page.getByLabel("What the runs end with").getByText("$1M–$2M"),
  ).toBeVisible();

  await page.locator('a[href*="field=monte_carlo.trials"]').click();
  await page.waitForURL(/#\/plan\/market\?.*field=monte_carlo\.trials/);
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.goto("#/tools/monte-carlo?run=p10");
  await expect(verdict).toBeVisible(SEARCH);
  await page.getByRole("link", { name: "Open in Ledger" }).click();
  await page.waitForURL(/#\/ledger\?.*market=trial-\d+/);
  const banner = page.getByText(
    /^random market \d+: the plan as a market tool ran it\./,
  );
  await expect(banner).toBeVisible();
  const replayed = await page.getByRole("table").first().innerText();
  await page.getByRole("link", { name: "Back to the plan" }).click();
  await page.waitForURL((url) => !url.hash.includes("market="));
  await expect(banner).toHaveCount(0);
  expect(await page.getByRole("table").first().innerText()).not.toBe(replayed);
  await page.goBack();
  await expect(banner).toBeVisible();
  await page.getByRole("button", { name: "The year after" }).click();
  await page.waitForURL(/year=\d+/);
  await page
    .getByRole("group", { name: "Show dollars as" })
    .getByRole("link", { name: "future dollars" })
    .click();
  await page.waitForURL(/basis=nominal/);
  expect(page.url()).toMatch(/market=trial-\d+/);
  await expect(banner).toBeVisible();

  await page.goto("#/tools/monte-carlo?run=planned");
  await expect(verdict).toBeVisible(SEARCH);
  await page.getByRole("link", { name: "Open in Ledger" }).click();
  await page.waitForURL(/#\/ledger/);
  expect(page.url()).not.toContain("market=");

  await page.goto("#/tools/historical");
  await expect(page.getByRole("heading", { name: "Historical" })).toBeVisible();
  await expect(page.getByText(/^survived [\d,]+ of [\d,]+$/)).toBeVisible(
    SEARCH,
  );
  await expect(page.getByRole("tab", { name: "By year" })).toHaveCount(0);
  await expectAccessible(page);
  const start = phone
    ? page.getByRole("button", { name: /^\d{4}/ }).first()
    : page.getByRole("row", { name: /^\d{4}/ }).first();
  const year = /\d{4}/.exec(await start.innerText())?.[0] ?? "";
  await start.click();
  await page.waitForURL(new RegExp(`run=(%22)?${year}`));
  await page.getByRole("link", { name: "Open in Ledger" }).click();
  await page.waitForURL(new RegExp(`market=(%22)?${year}`));
  await expect(
    page.getByText(new RegExp(`^retiring in ${year}: the plan`)),
  ).toBeVisible();

  await page.goto("#/ledger?market=1700");
  await expect(
    page.getByText(/retiring in 1700 cannot be replayed/),
  ).toBeVisible();
  await page.goto("#/ledger?market=p10");
  await expect(page.getByText(/no market is called p10/)).toBeVisible();
  await page.getByRole("link", { name: "Back to the plan" }).click();
  await expect(page.getByRole("heading", { name: "Ledger" })).toBeVisible();
});
