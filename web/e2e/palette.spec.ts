import {
  example,
  expect,
  expectAccessible,
  isPhone,
  seed,
  test,
} from "./support";

test("the palette finds pages, plans and actions; keys reach the tabs", async ({
  page,
}, testInfo) => {
  test.skip(isPhone(testInfo), "a phone has no keyboard to press them with");
  await seed(
    page,
    {
      "/early.toml": example("early-retiree.toml"),
      "/couple.toml": example("retired-couple.toml"),
    },
    "/early.toml",
    "#/overview?year=2030",
  );
  await expect(
    page.getByRole("heading", { name: "Overview", level: 1 }),
  ).toBeVisible();
  await page.keyboard.press("ControlOrMeta+k");
  const find = page.getByRole("combobox", {
    name: "Find a page, a plan or an action",
  });
  await find.fill("roth conv");
  await expect(page.getByRole("option").first()).toHaveText(
    "Tools · Roth ConversionsPage",
  );
  await expectAccessible(page);
  await page.keyboard.press("Enter");
  await page.waitForURL(/#\/tools\/roth-conversions\?.*year=2030/);
  await expect(
    page.getByRole("heading", { name: "Roth Conversions", level: 1 }),
  ).toBeFocused();

  await page
    .getByRole("button", { name: "Find a page, a plan or an action" })
    .click();
  await find.fill("open");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowUp");
  await expect(page.getByRole("option", { selected: true })).toHaveText(
    "Open couple.tomlPlan",
  );
  await page.keyboard.press("Enter");
  await expect(page.locator("header").getByText("couple.toml")).toBeVisible();

  await page.keyboard.press("2");
  await page.waitForURL(/#\/ledger/);
  await page.keyboard.press("5");
  await page.waitForURL(/#\/plan\/accounts/);
  await page.keyboard.press("?");
  await expect(
    page.getByRole("dialog", { name: "Keyboard shortcuts" }),
  ).toBeVisible();
  await expectAccessible(page);
  await page.keyboard.press("3");
  expect(page.url()).toMatch(/#\/plan\/accounts/);
  await page.keyboard.press("Escape");
  await page.goto("#/tools/tax-tables");
  await page.getByLabel("State").focus();
  await page.keyboard.press("1");
  expect(page.url()).toMatch(/tax-tables/);

  await page.goto("#/overview");
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Overview", level: 1 }),
  ).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to the page" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("main")).toBeFocused();
});
