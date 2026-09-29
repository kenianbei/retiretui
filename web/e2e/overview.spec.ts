import {
  SEARCH,
  example,
  expect,
  expectAccessible,
  seed,
  test,
} from "./support";

const FILES = { "/starter.toml": example("starter.toml") };

test("the Overview charts the plan, and a click chooses the year", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml", "#/overview?basis=nominal");
  const tabs = page.getByRole("tab");
  await expect(tabs).toHaveText(["Balances", "Net worth", "Income", "Markets"]);
  for (const name of ["Balances", "Net worth", "Income"]) {
    await page.getByRole("tab", { name }).click();
    const panel = page.getByRole("tabpanel");
    await expect(panel.locator(".recharts-surface")).toBeVisible();
    await expect(panel).toContainText("future dollars");
  }
  await page.getByRole("tab", { name: "Markets" }).click();
  const markets = page.getByRole("tabpanel");
  await expect(markets.locator(".recharts-area").first()).toBeVisible(SEARCH);
  await expect(markets).toContainText("today's dollars");
  await expectAccessible(page);

  await page.getByRole("tab", { name: "Net worth" }).click();
  const plot = page.getByRole("tabpanel").locator(".recharts-surface");
  await plot.scrollIntoViewIfNeeded();
  const box = await plot.boundingBox();
  if (!box) throw new Error("the chart has no box");
  await page.mouse.click(box.x + box.width * 0.6, box.y + box.height * 0.5);
  await page.waitForURL(/year=\d{4}/);
  const year = /year=(\d{4})/.exec(page.url())?.[1] ?? "";
  await expect(page.locator("#this-year")).toContainText(year);
  await page.getByRole("link", { name: "Ledger" }).first().click();
  await expect(page.getByText(`${year} Flows · future dollars`)).toBeVisible();

  await page.goto(`#/overview?year=${year}`);
  await page.getByRole("tab", { name: "Net worth" }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "Income", selected: true }),
  ).toBeVisible();
  expect(page.url()).toContain(`year=${year}`);
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
  await expect(page.getByText(/^Short \$.* from \d{4}$/)).toBeVisible();
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
  await expectAccessible(page);
  await problems
    .getByRole("link", { name: /Balance: must not be negative/ })
    .click();
  await page.waitForURL(/#\/plan\/accounts\?.*field=balance/);
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
