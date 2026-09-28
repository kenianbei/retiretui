import {
  SEARCH,
  compareWith,
  example,
  expect,
  expectAccessible,
  isPhone,
  openPlan,
  rowNamed,
  searchesDone,
  seed,
  test,
} from "./support";

test("plans compared against a baseline, and a written scenario compared at once", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  await seed(
    page,
    {
      "/early.toml": example("early-retiree.toml"),
      "/couple.toml": example("retired-couple.toml"),
      "/starter.toml": example("starter.toml"),
    },
    "/early.toml",
    "#/compare",
  );
  await expect(page.getByText(/Compare with adds one/)).toBeVisible();
  const row = (name: string) => rowNamed(page, testInfo, name);
  const tab = (name: string) =>
    page
      .getByRole("link", { name, exact: true })
      .filter({ visible: true })
      .first();
  const hash = () => decodeURIComponent(new URL(page.url()).hash);

  await compareWith(page, "couple.toml");
  expect(hash()).toContain("couple.toml");
  await expect(row("couple.toml")).toBeVisible();
  if (!phone) {
    await expect(
      page.getByRole("row", { name: /^couple\.toml.*\d%/ }),
    ).toBeVisible(SEARCH);
    await expect(
      page.getByRole("row", { name: /^early\.toml.*\d%/ }),
    ).toBeVisible(SEARCH);
  }
  await expectAccessible(page);

  await page.getByRole("button", { name: "Difference" }).click();
  await page.waitForURL(/difference=true/);
  await expect(page.getByText(/^Plans · against early\.toml/)).toBeVisible();
  if (!phone) {
    await expect(
      page.getByRole("row", { name: /^couple\.toml [+-]\$/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("row", { name: /^couple\.toml.*(pts|same)/ }),
    ).toBeVisible(SEARCH);
  }
  await row("couple.toml").click();
  await page.waitForURL(/plan=/);
  await expect(
    page.getByText("Changes · couple.toml against early.toml"),
  ).toBeVisible();
  await page.getByRole("button", { name: "Make baseline" }).click();
  await page.waitForURL(/baseline=/);
  await expect(page.getByText(/^Plans · against couple\.toml/)).toBeVisible();
  await expect(page.getByText("The baseline.")).toBeVisible();

  await page.getByLabel("Metric").selectOption("taxes");
  await page.waitForURL(/metric=taxes/);
  await expect(
    page.getByText(/^Taxes by year · against couple\.toml/),
  ).toBeVisible();
  if (!phone) {
    await expect(
      page.getByRole("columnheader", { name: /^Taxes \d{4}$/ }),
    ).toBeVisible();
  }
  await expect(
    page.getByLabel(/^Taxes by year/).locator(".recharts-line"),
  ).toHaveCount(2);
  await page.getByRole("tab", { name: "By year" }).click();
  await page.waitForURL(/view=table/);
  const years = page.getByRole("table", { name: /^Taxes by year/ });
  await expect(years.getByRole("columnheader", { name: "Year" })).toBeVisible();
  await years.getByRole("row", { name: /^2031/ }).click();
  await page.waitForURL(/year=2031/);

  await tab("Plan").click();
  await page.waitForURL(/#\/plan\//);
  expect(hash()).toContain("couple.toml");
  await tab("Compare").click();
  await page.waitForURL(/#\/compare/);
  await expect(row("couple.toml")).toBeVisible();
  await page.reload();
  await expect(row("couple.toml")).toBeVisible();

  await row("couple.toml").click();
  await page.getByRole("button", { name: "Open" }).click();
  await expect(row("early.toml")).toBeVisible();
  await expect.poll(hash).toContain("early.toml");
  expect(hash()).not.toContain("couple.toml");
  await expect(page.locator("header").getByText("couple.toml")).toBeVisible();

  await openPlan(page, "starter.toml");
  await expect.poll(hash).not.toContain("with=");
  await expect(page.getByText(/Compare with adds one/)).toBeVisible();

  await openPlan(page, "early.toml");
  await expect(page.locator("header").getByText("early.toml")).toBeVisible();
  await page.goto("#/tools/roth-conversions");
  await expect(
    page.getByRole("heading", { name: /^Conversions \(/ }),
  ).toBeVisible(SEARCH);
  await searchesDone(page);
  await page.getByRole("button", { name: "Write as a scenario" }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: "Write" })
    .click();
  const wrote = page.getByText(/^Wrote early-ladder-\d+\.toml, compared\./);
  await expect(wrote).toBeVisible();
  const ladder =
    /early-ladder-\d+\.toml/.exec(await wrote.innerText())?.[0] ?? "";
  await page
    .getByRole("link", { name: "Compare", exact: true })
    .filter({ visible: true })
    .last()
    .click();
  await page.waitForURL(/#\/compare/);
  await row(ladder).click();
  await expect(
    page.getByText(`Changes · ${ladder} against early.toml`),
  ).toBeVisible();
  await expect(page.getByText(/^Conversions/).first()).toBeVisible();
});
