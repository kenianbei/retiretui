import {
  example,
  expect,
  expectAccessible,
  FULL_MONEY,
  isPhone,
  rowNamed,
  SEARCH,
  scrollDown,
  searchesDone,
  seed,
  test,
} from "./support";

const RETIRED = example("retired-couple.toml");

test("both ceilings are found, one taken and written, and another target searched", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  await seed(
    page,
    { "/retired.toml": RETIRED },
    "/retired.toml",
    "#/tools/spending-ceiling",
  );
  await expect(
    page.getByRole("heading", { name: "Spending Ceiling" }),
  ).toBeVisible();
  await expect(page.getByText(/in the plan's own market/)).toBeVisible();

  const ceilings = page.getByRole("region", { name: "Ceilings" });
  const expenses = page.getByRole("region", { name: /^Expenses/ });
  const take = expenses.getByRole("button", { name: "Take this ceiling" });
  await expect(take).toBeVisible(SEARCH);
  await searchesDone(page);
  await expectAccessible(page);
  await expect(
    page.getByRole("heading", { name: "Expenses (in 90% of markets)" }),
  ).toBeVisible();
  await expect(expenses.getByText("Living expenses")).toBeVisible();
  await expect(expenses.getByText("Medicare premiums")).toHaveCount(0);
  if (!phone) {
    const rows = ceilings.getByRole("table").locator("tbody tr");
    await expect(rows).toHaveCount(3);
    await expect(rows.nth(0)).toContainText("Current");
    await expect(rows.nth(0)).toContainText("$114,000");
    await expect(rows.nth(1)).toContainText("In its own market");
    await expect(rows.nth(2)).toContainText("In 90% of markets");
    await expect(rows.nth(2)).toContainText(FULL_MONEY);
  }

  const top = await scrollDown(page);
  await rowNamed(page, testInfo, "In its own market").dispatchEvent("click");
  await page.waitForURL(/ceiling=planned/);
  await expect(page.getByText(/spends it all by its end/)).toBeVisible();
  const highlighted = await page.evaluate(() => window.scrollY);
  expect(highlighted, "a highlight leaves the reader there").toBe(top);
  await expect(
    page.getByRole("link", { name: "Edit Leave at least" }),
  ).toBeVisible();
  await page.reload();
  await expect(take).toBeVisible(SEARCH);
  await searchesDone(page);
  await expect(
    page.getByRole("heading", { name: "Expenses (in its own market)" }),
  ).toBeVisible();
  await take.click();
  const asked = page.getByRole("alertdialog");
  await expect(asked.getByText(/^Set flexible spending to \$/)).toBeVisible();
  await asked.getByRole("button", { name: "Take", exact: true }).click();
  await expect(page.getByText(/^flexible spending is now \$/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Undo" }).first(),
  ).toBeEnabled();

  const target = page.getByRole("region", { name: "Target" });
  await expect(target).toContainText("90%, the default");
  await target.getByRole("link", { name: "Edit" }).click();
  await page.waitForURL(/edit=true/);
  const sheet = page.getByRole("dialog");
  await sheet.getByRole("textbox", { name: "Target success" }).fill("75%");
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(target).toContainText("75%");
  await rowNamed(page, testInfo, "In 75% of markets").click();
  await page.waitForURL(/ceiling=target/);
  await searchesDone(page);

  await expenses.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(asked.getByText(/save first/)).toBeVisible();
  await asked.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await searchesDone(page);
  await expenses.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(
    page.getByText(/^Wrote retired-spending-target\.toml, compared\./),
  ).toBeVisible();
});

test("a plan with nothing flexible says there is nothing to scale", async ({
  page,
}) => {
  const fixed = RETIRED.replace("essential = true\n", "").replaceAll(
    /\[\[expenses\]\]\n(?:.+\n)*?amount = \d+\n/g,
    "$&essential = true\n",
  );
  await seed(
    page,
    { "/fixed.toml": fixed },
    "/fixed.toml",
    "#/tools/spending-ceiling",
  );
  await expect(
    page.getByRole("alert").getByText(/no flexible spending to scale/),
  ).toBeVisible(SEARCH);
  await expect(page.getByRole("button", { name: /^Take/ })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Target" })).toBeVisible();
});
