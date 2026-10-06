import {
  example,
  expect,
  expectAccessible,
  isPhone,
  rowNamed,
  SEARCH,
  seed,
  test,
} from "./support";

const FILES = { "/starter.toml": example("starter.toml") };

test("the Overview charts the plan a chart at a time, and a click opens that year in the Ledger", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml", "#/overview?basis=nominal");
  const chart = (name: string) =>
    page.getByRole("region", { name, exact: true });
  const tabs = page.getByRole("navigation", { name: "Chart" });
  const balances = chart("Balances by tax treatment");
  await expect(balances.locator(".recharts-surface")).toBeVisible();
  await expect(balances).toContainText("future dollars");
  await expect(balances).toContainText("pre-tax");
  await expectAccessible(page);

  await tabs.getByRole("link", { name: "Income against taxes" }).click();
  await page.waitForURL(/chart=income/);
  await expect(
    chart("Income against taxes").locator(".recharts-surface"),
  ).toBeVisible();
  await tabs
    .getByRole("link", { name: "Net worth through random markets" })
    .click();
  const markets = chart("Net worth through random markets");
  await expect(markets.locator(".recharts-area").first()).toBeVisible(SEARCH);
  await expect(markets).toContainText("today's dollars");
  await expectAccessible(page);

  await tabs.getByRole("link", { name: "Net worth", exact: true }).click();
  await page.waitForURL(/chart=net-worth/);
  await page.reload();
  const plot = chart("Net worth").locator(".recharts-surface");
  await plot.scrollIntoViewIfNeeded();
  const box = await plot.boundingBox();
  if (!box) throw new Error("the chart has no box");
  await page.mouse.click(box.x + box.width * 0.6, box.y + box.height * 0.5);
  await page.waitForURL(/#\/ledger\?.*year=\d{4}/);
  const year = /year=(\d{4})/.exec(page.url())?.[1] ?? "";
  await expect(
    page.getByRole("heading", {
      level: 2,
      name: new RegExp(`^${year} · .* · future dollars$`),
    }),
  ).toBeVisible();
});

test("the Overview holds no year, and carries the one it was given on", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml", "#/overview?year=2045");
  await expect(page.getByRole("term")).toHaveCount(4);
  await expect(page.getByRole("button", { name: /^The year/ })).toHaveCount(0);
  await expect(page.getByText(/What to do/)).toHaveCount(0);
  const totals = page.getByRole("region", { name: "Over the plan" });
  const before = await totals.textContent();
  await page.keyboard.press("ArrowRight");
  expect(page.url()).toContain("year=2045");
  expect(await totals.textContent()).toBe(before);
  await page.getByRole("link", { name: "Ledger", exact: true }).click();
  await page.waitForURL(/#\/ledger\?.*year=2045/);
});

test("the years add up, each total leading to its page or tool", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml");
  const totals = page.getByRole("region", { name: "Over the plan" });
  await expect(totals.getByRole("listitem")).toHaveCount(7);
  await expect(totals).toContainText("today's dollars");
  const income = totals.getByRole("link", { name: /^Income/ });
  await expect(income).toContainText(/salary \d+%/);
  const today = await income.textContent();
  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  await expect(income).not.toHaveText(today ?? "");
  await expect(totals).toContainText("future dollars");
  await expectAccessible(page);

  await totals.getByRole("link", { name: /^Withdrawals/ }).click();
  await page.waitForURL(/#\/tools\/withdrawal-order\?.*basis=nominal/);
  await page.goBack();
  await totals.getByRole("link", { name: /^Taxes/ }).click();
  await page.waitForURL(/#\/tools\/tax-tables/);
  await page.goBack();
  await income.click();
  await page.waitForURL(/#\/plan\/income/);
});

test("what the plan rests on leads to the field it is edited at", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml");
  const rests = page.getByRole("region", { name: "Rests on" });
  await expect(rests.getByRole("link").first()).toContainText(/to age \d+/);
  await rests.getByRole("link", { name: /^Inflation/ }).click();
  await page.waitForURL(/#\/plan\/settings\?.*field=inflation/);
});

test("what the plan could spend is said among what could do better", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml");
  const better = page.getByRole("region", { name: "Could do better" });
  const row = better.getByRole("link", { name: /\d+% of markets/ });
  await expect(row).toBeVisible(SEARCH);
  await expect(row).toContainText(/^(Could spend|Spend|Spends) /);
  await row.click();
  await page.waitForURL(/#\/tools\/spending-ceiling/);
});

test("the strip reads the plan, and a plan that runs short says where", async ({
  page,
}) => {
  const short = example("starter.toml").replace(
    "amount = 24000",
    "amount = 240000",
  );
  await seed(page, { "/short.toml": short }, "/short.toml");
  await expect(page.getByRole("term")).toHaveText([
    "Money lasts",
    "Success",
    "Lowest after retiring",
    "Ends with",
  ]);
  const note = page.getByText(/^Runs short from \d{4}: /);
  await expect(note).toBeVisible();
  const year = /from (\d{4})/.exec((await note.textContent()) ?? "")?.[1];
  await expect(
    page.getByText("Short from the start", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/unfunded/)).toHaveCount(0);
  await expect(page.getByRole("term")).toHaveCount(4);
  await expect(page.getByText("through random markets")).toBeVisible(SEARCH);
  await expectAccessible(page);

  await page
    .getByRole("link", { name: `${year ?? ""} in the Ledger` })
    .first()
    .click();
  await page.waitForURL(new RegExp(`#/ledger\\?.*year=${year ?? ""}`));
  await page.goBack();
  await page.getByRole("link", { name: "Expenses", exact: true }).click();
  await page.waitForURL(/#\/plan\/expenses/);
});

test("an issue leads to its field", async ({ page }) => {
  const broken = example("starter.toml").replace(
    /balance = \d+/,
    "balance = -1",
  );
  await seed(page, { "/broken.toml": broken }, "/broken.toml");
  const problems = page.getByRole("region", { name: "This plan has 1 issue" });
  await expect(problems).toContainText("Its figures show once they are fixed.");
  await expect(page.getByRole("term")).toHaveCount(0);
  await expect(
    page.getByRole("group", { name: "Show dollars as" }),
  ).toHaveCount(0);
  await expectAccessible(page);
  await problems
    .getByRole("link", { name: /Balance: must not be negative/ })
    .click();
  await page.waitForURL(/#\/plan\/accounts\?.*field=balance/);
});

test("the worst historical start the plan does not survive leads to it", async ({
  page,
}, testInfo) => {
  const spending = example("starter.toml").replace(
    "amount = 24000",
    "amount = 60000",
  );
  await seed(page, { "/spending.toml": spending }, "/spending.toml");
  const attention = page.getByRole("region", { name: "Needs attention" });
  const row = attention.getByRole("link", {
    name: /^Fails from a \d{4} start · [\d,]+ of 155 fail/,
  });
  await expect(row).toBeVisible(SEARCH);
  const year = /\d{4}/.exec((await row.textContent()) ?? "")?.[0] ?? "";
  await row.click();
  await page.waitForURL(new RegExp(`#/tools/historical\\?.*run=(%22)?${year}`));
  await expect(rowNamed(page, testInfo, year)).toHaveAttribute(
    isPhone(testInfo) ? "aria-pressed" : "aria-selected",
    "true",
  );
});

test("a milestone leads to its year in the Ledger", async ({ page }) => {
  await seed(
    page,
    { "/couple.toml": example("mid-career-couple.toml") },
    "/couple.toml",
  );
  const milestones = page.getByRole("region", { name: "Milestones" });
  await milestones.getByRole("link", { name: /^2043 Priya retires/ }).click();
  await page.waitForURL(/#\/ledger\?.*year=2043/);
});
