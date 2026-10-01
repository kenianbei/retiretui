import {
  SEARCH,
  SIGNED_MONEY,
  example,
  expect,
  expectAccessible,
  isPhone,
  rowNamed,
  searchesDone,
  seed,
  test,
} from "./support";

const BEST = "Roth, deferred, taxable, HSA";

test("the orders are ranked, the best said on the Overview, and one taken and written", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  await seed(
    page,
    { "/early.toml": example("early-retiree.toml") },
    "/early.toml",
  );
  const card = page.getByRole("region", { name: "Could do better" });
  const said = card.getByRole("link", {
    name: /^Withdraw in the order Roth, deferred, taxable, HSA: ends \+\$/,
  });
  await expect(said).toBeVisible(SEARCH);
  await said.click();
  await page.waitForURL(/#\/tools\/withdrawal-order/);
  await expect(
    page.getByRole("heading", { name: "Withdrawal Order" }),
  ).toBeVisible();
  await expect(page.getByText(/which is spent first changes/)).toBeVisible();

  const orders = page.getByRole("region", { name: "Orders" });
  const take = orders.getByRole("button", { name: "Take this order" });
  await expect(take).toBeVisible(SEARCH);
  await searchesDone(page);
  await expectAccessible(page);
  if (!phone) {
    const rows = orders.getByRole("table").locator("tbody tr");
    await expect(rows).toHaveCount(6);
    await expect(rows.nth(0)).toContainText("Current");
    await expect(rows.nth(0)).not.toContainText(/[+-]\$/);
    await expect(rows.nth(1)).toContainText(BEST);
    await expect(rows.nth(1)).toContainText(SIGNED_MONEY);
  }

  const another = "Deferred, Roth, taxable, HSA";
  await rowNamed(page, testInfo, another).click();
  await page.waitForURL(/order=deferred-roth-taxable-hsa/);
  await page.reload();
  await expect(take).toBeVisible(SEARCH);
  await searchesDone(page);
  await take.click();
  const asked = page.getByRole("alertdialog");
  await expect(
    asked.getByText(`Withdraw in this order? ${another}.`),
  ).toBeVisible();
  await asked.getByRole("button", { name: "Take", exact: true }).click();
  await expect(
    page.getByText("now withdrawing in the order deferred, Roth, taxable, HSA"),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Undo" }).first(),
  ).toBeEnabled();

  await searchesDone(page);
  await orders.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(asked.getByText(/save first/)).toBeVisible();
  await asked.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await searchesDone(page);
  await orders.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(
    page.getByText(/^Wrote early-order-[a-z-]+\.toml, compared\./),
  ).toBeVisible();
});

test("a plan with one kind of account to withdraw from says there is nothing to order", async ({
  page,
}) => {
  const oneKind = example("early-retiree.toml").replace(
    /withdrawal_order = .*/,
    'withdrawal_order = ["taxable"]',
  );
  await seed(
    page,
    { "/one.toml": oneKind },
    "/one.toml",
    "#/tools/withdrawal-order",
  );
  await expect(
    page.getByRole("alert").getByText(/there is nothing to order/),
  ).toBeVisible(SEARCH);
  await expect(page.getByRole("button", { name: /^Take/ })).toHaveCount(0);
});
