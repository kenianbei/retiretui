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
  if (!isPhone(testInfo))
    await expect(
      page
        .getByRole("region", { name: "Checking" })
        .getByText("Earns nothing", { exact: true }),
    ).toHaveClass(/text-muted-foreground/);
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

test("an empty domain says what it holds before Add, and a form leads with the name", async ({
  page,
}) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/plan/cliffs",
  );
  const main = page.locator("main");
  const holds = main.getByText(/^Costs that start once your income passes/);
  const add = main.getByRole("link", { name: "Add" });
  await expect(add).toBeVisible();
  const [said, offered] = await Promise.all([
    holds.boundingBox(),
    add.boundingBox(),
  ]);
  expect((said?.y ?? Infinity) < (offered?.y ?? 0)).toBe(true);
  await expectAccessible(page);

  await page.goto("#/plan/accounts?edit=0");
  await expect(
    page.getByRole("dialog").getByRole("textbox").first(),
  ).toHaveAccessibleName("Name");
});

test("a workplace plan asks when its job is left, and only then whether it is public safety's", async ({
  page,
}) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/plan/accounts?edit=0",
  );
  const sheet = page.getByRole("dialog");
  const left = sheet.getByLabel("Job left", { exact: true });
  const safety = sheet.getByRole("checkbox", { name: /^Public safety/ });
  await expect(sheet.getByLabel("ID", { exact: true })).toHaveValue("checking");
  await expect(left).toHaveCount(0);

  await page.goto("#/plan/accounts?edit=1");
  await expect(sheet.getByLabel("ID", { exact: true })).toHaveValue("401k-sam");
  await expect(left).toBeVisible();
  await expect(safety).toHaveCount(0);
  await left.selectOption({ label: "Event" });
  await expect(safety).toHaveCount(0);
  await sheet
    .getByLabel(/^The event it follows/)
    .selectOption({ label: "Sam retires" });
  await expect(safety).toBeVisible();
  await safety.check();
  await expectAccessible(page);
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(sheet).toHaveCount(0);
  await expect(page.locator("main")).toContainText("Public safety");
});
