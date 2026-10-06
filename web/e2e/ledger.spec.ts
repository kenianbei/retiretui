import type { Page } from "@playwright/test";

import {
  example,
  expect,
  expectAccessible,
  isPhone,
  seed,
  test,
} from "./support";

/** The year shown in full, by its card's heading. */
function yearShown(page: Page, year: string, unit = "today's dollars") {
  return page.getByRole("heading", {
    level: 2,
    name: new RegExp(`^${year} · Sam turns \\d+ · ${unit}$`),
  });
}

/** What a side of the year's money comes to. */
function totalOf(page: Page, side: string) {
  return page.getByRole("group", { name: side }).locator("dd").last();
}

test("the Ledger shows a year in full beside the list of years", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/ledger?year=2040",
  );
  await expect(yearShown(page, "2040")).toBeVisible();
  await expect(page.getByRole("table", { name: /Flows/ })).toBeVisible();
  const lived = await totalOf(page, "Money in").textContent();
  expect(lived).toMatch(/^\$[\d,]+$/);
  await expect(totalOf(page, "Money out")).toHaveText(lived ?? "");
  await expect(page.getByLabel("Paid").getByText("Total")).toBeVisible();

  const years = page.getByRole("table", { name: /^Years/ });
  if (isPhone(testInfo)) {
    await expect(years, "a phone reads the year alone").toBeHidden();
    await page.getByRole("button", { name: "The year after" }).click();
  } else {
    const rows = years.locator("tbody tr");
    await expect(rows.nth(20)).toBeVisible();
    await expect(rows.nth(14)).toHaveAttribute("aria-selected", "true");
    await rows.nth(15).click();
    await expect(rows.nth(15)).toHaveAttribute("aria-selected", "true");
  }
  await page.waitForURL(/year=2041/);
  await expect(yearShown(page, "2041")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(yearShown(page, "2042")).toBeVisible();
  await expectAccessible(page);

  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  await expect(totalOf(page, "Money in")).not.toHaveText(lived ?? "");
  await page.reload();
  await expect(yearShown(page, "2042", "future dollars")).toBeVisible();
});

test("the Ledger's table turns through its column sets, and a row leads to its year", async ({
  page,
}) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/ledger?year=2045",
  );
  await page
    .getByRole("group", { name: "Show" })
    .getByRole("link", { name: "Table" })
    .click();
  await page.waitForURL(/view=table/);
  const table = page.getByRole("table", { name: /year by year/ });
  const rows = table.locator("tbody tr");
  await expect(rows.nth(20)).toBeVisible();
  await expect(rows.nth(19)).toHaveAttribute("aria-selected", "true");
  await expect(yearShown(page, "2045")).toHaveCount(0);

  const scroller = table.locator("xpath=..");
  await scroller.evaluate((element) => {
    element.scrollLeft = 400;
  });
  const pinned = await table.locator("tbody tr td").first().boundingBox();
  const frame = await scroller.boundingBox();
  expect(pinned?.x ?? 0).toBeLessThanOrEqual((frame?.x ?? 0) + 2);

  const columns = page.getByLabel("Columns");
  const header = (name: string) =>
    table.getByRole("columnheader", { name, exact: true });
  await expect(header("Roth")).toBeVisible();
  await columns.selectOption({ label: "Tax figures" });
  await page.waitForURL(/columns=tax/);
  await expect(header("MAGI")).toBeVisible();
  await expect(header("Roth")).toHaveCount(0);
  await expect(header("Net worth")).toBeVisible();
  await expectAccessible(page);
  await columns.selectOption({ label: "Balances by account" });
  await page.waitForURL(/columns=accounts/);
  await expect(header("Sam's Roth IRA")).toBeVisible();
  await columns.selectOption({ label: "Balances by treatment" });
  await page.waitForURL((url) => !url.hash.includes("columns="));

  await rows.nth(21).click();
  await page.waitForURL(
    (url) => url.hash.includes("year=2047") && !url.hash.includes("view="),
  );
  await expect(yearShown(page, "2047")).toBeVisible();
});

test("the Ledger says what to do in its year, in the dollars shown", async ({
  page,
}) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/ledger?year=2045",
  );
  const first = page
    .getByRole("list", { name: "To do" })
    .getByRole("listitem")
    .first();
  await expect(yearShown(page, "2045")).toBeVisible();
  const today = await first.textContent();
  await expect(page.getByText(/^So far: /)).toBeVisible();
  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  await expect(yearShown(page, "2045", "future dollars")).toBeVisible();
  await expect(first).not.toHaveText(today ?? "");
  await page.getByRole("button", { name: "The year after" }).click();
  await expect(yearShown(page, "2046", "future dollars")).toBeVisible();
  await page.getByRole("link", { name: "Tax tables for 2046" }).click();
  await page.waitForURL(/#\/tools\/tax-tables\?.*year=2046/);
  expect(page.url()).toContain("basis=nominal");
});

test("the marked years either side are stepped to, and say what marks them", async ({
  page,
}) => {
  await seed(
    page,
    { "/couple.toml": example("retired-couple.toml") },
    "/couple.toml",
    "#/ledger?year=2029",
  );
  const before = page.getByRole("button", { name: "The marked year before" });
  const after = page.getByRole("button", { name: "The marked year after" });
  const marks = page.getByRole("list", { name: /^(Milestones|To watch)$/ });
  await expect(marks).toHaveCount(0);
  await after.click();
  await page.waitForURL(/year=2031/);
  await expect(marks.first()).toBeVisible();
  const lived = page.getByRole("group", { name: "Money in" });
  await expect(lived.getByText(/^RMD from /).first()).toBeVisible();
  await expect(lived.getByText("Withdrawn", { exact: true })).toBeVisible();
  const spent = page.getByRole("group", { name: "Money out" });
  await expect(spent.locator("dt").first()).not.toHaveText(
    /^(Spending|Essential|Flexible)$/,
  );
  await expect(spent.getByText("Essential", { exact: true })).toBeVisible();
  await expect(spent.getByText("Tax", { exact: true })).toBeVisible();
  await before.click();
  await page.waitForURL(/year=2027/);
  await expect(page.getByRole("list", { name: "To watch" })).toBeVisible();
  await before.click();
  await page.waitForURL(/year=2026/);
  await expect(before).toBeDisabled();
});
