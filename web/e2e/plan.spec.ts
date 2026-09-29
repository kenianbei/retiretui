import {
  STATEMENT,
  example,
  expect,
  expectAccessible,
  isPhone,
  openPlan,
  seed,
  test,
} from "./support";

test("a domain's items are tabled in any column's order", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/plan/accounts",
  );
  await expect(
    page.getByRole("heading", { name: "Accounts", level: 1 }),
  ).toBeVisible();
  await expectAccessible(page);
  if (isPhone(testInfo)) {
    await page
      .getByLabel("Sort by")
      .selectOption({ label: "Balance, highest first" });
    await expect(page.locator("main ul.divide-y li").first()).toContainText(
      "401",
    );
    return;
  }
  const firstBalance = page.locator("tbody tr").first().locator("td").last();
  await page.getByRole("button", { name: "Balance" }).click();
  await expect(firstBalance).toHaveText("$3,000");
  await page.getByRole("button", { name: "Balance" }).click();
  await expect(firstBalance).toHaveText("$12,000");
  await page.getByRole("button", { name: "Balance" }).click();
  await expect(firstBalance).toHaveText("$5,000");
});

test("edits not applied are asked about before they are left", async ({
  page,
}) => {
  await seed(
    page,
    { "/early.toml": example("early-retiree.toml") },
    "/early.toml",
    "#/plan/accounts",
  );
  for (const [base, address, field] of [
    ["#/plan/settings", "#/plan/settings?edit=0", "Inflation"],
    [
      "#/tools/roth-conversions",
      "#/tools/roth-conversions?edit=true",
      "Headroom",
    ],
  ] as const) {
    await page.goto(base);
    await page.getByRole("heading", { level: 1 }).waitFor();
    await page.goto(address);
    const sheet = page.getByRole("dialog");
    await sheet.getByLabel(field, { exact: true }).fill("7");
    await expectAccessible(page);
    await page.goBack();
    await expect(
      page.getByRole("alertdialog").getByText("Apply your edits first?"),
    ).toBeVisible();
    await page.getByRole("button", { name: "Discard edits" }).click();
    await expect(sheet).toHaveCount(0);
  }
});

test("a statement's earnings are recorded on the person it names", async ({
  page,
}) => {
  const starter = example("starter.toml");
  const born = starter.replace("birth = 1996-03-10", "birth = 1975-06-14");
  await seed(
    page,
    {
      "/born.toml": born,
      "/other.toml": starter,
      "/what-if.toml": 'schema = 1\nbase = "other.toml"\n',
    },
    "/born.toml",
    "#/plan/people?item=0",
  );
  await expect(
    page.getByRole("button", { name: "Import statement" }),
  ).toBeVisible();
  await page.getByLabel("Social Security statement").setInputFiles(STATEMENT);
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: /recorded 3 years of earnings for Sam/ }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Undo" }).first().click();
  await expect(
    page.getByRole("button", { name: "Undo" }).first(),
  ).toBeDisabled();

  await openPlan(page, "other.toml");
  await page.getByRole("button", { name: "Discard edits" }).click();
  await expect(page.locator("header").getByText("other.toml")).toBeVisible();
  await page.goto("#/plan/people?item=0");
  await page.getByLabel("Social Security statement").setInputFiles(STATEMENT);
  await expect(
    page.getByRole("alert").filter({ hasText: /was born 1996-03-10/ }),
  ).toBeVisible();

  await openPlan(page, "what-if.toml");
  await expect(page.getByText("Scenario · read-only").first()).toBeAttached();
  await page.goto("#/plan/people?item=0");
  await expect(page.getByRole("link", { name: "Edit" }).first()).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Import statement" }),
  ).toHaveCount(0);
});
