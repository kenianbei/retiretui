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

test("the Overview charts the plan at once, and a click chooses the year", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml", "#/overview?basis=nominal");
  const chart = (name: string) =>
    page.getByRole("region", { name, exact: true });
  for (const name of ["Balances", "Net worth", "Income & tax"]) {
    await expect(chart(name).locator(".recharts-surface")).toBeVisible();
    await expect(chart(name)).toContainText("future dollars");
  }
  const markets = chart("Market runs");
  await expect(markets.locator(".recharts-area").first()).toBeVisible(SEARCH);
  await expect(markets).toContainText("today's dollars");
  await expectAccessible(page);

  const plot = chart("Net worth").locator(".recharts-surface");
  await plot.scrollIntoViewIfNeeded();
  const box = await plot.boundingBox();
  if (!box) throw new Error("the chart has no box");
  await page.mouse.click(box.x + box.width * 0.6, box.y + box.height * 0.5);
  await page.waitForURL(/year=\d{4}/);
  const year = /year=(\d{4})/.exec(page.url())?.[1] ?? "";
  await expect(page.locator("#this-year")).toContainText(year);
  await page.getByRole("link", { name: "Ledger" }).first().click();
  await expect(page.getByText(`${year} Flows · future dollars`)).toBeVisible();
});

test("the year steps by button and key, within the plan's years", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml");
  const heading = page.locator("#this-year");
  await expect(heading).toContainText(/\d{4}/);
  const first = Number(/\d{4}/.exec((await heading.textContent()) ?? "")?.[0]);
  await page.getByRole("button", { name: "The year after" }).click();
  await expect(heading).toContainText(String(first + 1));
  expect(page.url()).toContain(`year=${String(first + 1)}`);
  await page.keyboard.press("ArrowLeft");
  await expect(heading).toContainText(String(first));
  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  expect(page.url()).toContain(`year=${String(first)}`);
  await page.getByRole("link", { name: "Ledger" }).first().click();
  await page.waitForURL(/ledger.*basis=nominal/);

  await page.goto("#/overview?year=1900");
  await expect(heading).toContainText(String(first));
  await expect(
    page.getByRole("button", { name: "The year before" }),
  ).toBeDisabled();
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
    "Ends with",
    "Lifetime taxes",
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

test("this year's actions are in the dollars shown", async ({ page }) => {
  await seed(page, FILES, "/starter.toml", "#/overview?year=2045");
  const actions = page.getByRole("region", { name: /^What to do in 2045/ });
  const today = await actions.getByRole("listitem").first().textContent();
  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  await expect(actions.getByRole("listitem").first()).not.toHaveText(
    today ?? "",
  );
});
