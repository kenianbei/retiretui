import {
  example,
  expect,
  expectAccessible,
  isPhone,
  seed,
  test,
} from "./support";

test("the sidebar says how many items each of the plan's pages holds", async ({
  page,
}, testInfo) => {
  test.skip(isPhone(testInfo), "a phone has no sidebar");
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/plan/accounts",
  );
  const nav = page.getByRole("navigation", { name: "Main" });
  const accounts = nav.getByRole("link", { name: /^Accounts/ });
  await expect(page.locator("tbody tr").first()).toBeVisible();
  const held = await page.locator("tbody tr").count();
  expect(held).toBeGreaterThan(1);
  await expect(accounts).toHaveText(`Accounts${held}`);
  await expect(nav.getByRole("link", { name: /^Settings/ })).toHaveText(
    "Settings",
  );
  await expectAccessible(page);

  await page.getByRole("button", { name: "Delete" }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: "Delete" })
    .click();
  await expect(accounts).toHaveText(`Accounts${held - 1}`);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(accounts).toHaveText(`Accounts${held}`);

  await page.goto("#/tools/roth-conversions");
  const tools = nav.getByRole("link", { name: /^SSA Benefits/ });
  await expect(tools).toHaveText("SSA Benefits");
});

test("a group's tab comes back to the page last shown in it", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/tools/historical",
  );
  const nav = page.getByRole("navigation", { name: "Main" });
  const tab = (name: string) => nav.getByRole("link", { name, exact: true });
  await expect(
    page.getByRole("heading", { name: "Historical", level: 1 }),
  ).toBeVisible();
  await tab("Tools").click();
  await expect(page, "an open group's own tab stays").toHaveURL(
    /#\/tools\/historical/,
  );
  await tab("Plan").click();
  await page.waitForURL(/#\/plan\/accounts/);
  await tab("Tools").click();
  await page.waitForURL(/#\/tools\/historical/);

  await page.goto("#/plan/expenses");
  await expect(
    page.getByRole("heading", { name: "Expenses", level: 1 }),
  ).toBeVisible();
  await tab("Ledger").click();
  await page.waitForURL(/#\/ledger/);
  if (isPhone(testInfo)) {
    await tab("Plan").click();
  } else {
    await page.keyboard.press("5");
  }
  await page.waitForURL(/#\/plan\/expenses/);
});
