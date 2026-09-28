import { example, expect, expectAccessible, seed, test } from "./support";

const FILES = { "/starter.toml": example("starter.toml") };

test("the Overview charts the plan, and a click chooses the year", async ({
  page,
}) => {
  await seed(page, FILES, "/starter.toml", "#/overview?basis=nominal");
  const tabs = page.getByRole("tab");
  await expect(tabs).toHaveText([
    "Balances",
    "Net worth",
    "Income & tax",
    "Market runs",
  ]);
  for (const name of ["Balances", "Net worth", "Income & tax"]) {
    await page.getByRole("tab", { name }).click();
    const panel = page.getByRole("tabpanel");
    await expect(panel.locator(".recharts-surface")).toBeVisible();
    await expect(panel).toContainText("future dollars");
  }
  await page.getByRole("tab", { name: "Market runs" }).click();
  const markets = page.getByRole("tabpanel");
  await expect(markets.locator(".recharts-area").first()).toBeVisible({
    timeout: 90_000,
  });
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
    page.getByRole("tab", { name: "Income & tax", selected: true }),
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
