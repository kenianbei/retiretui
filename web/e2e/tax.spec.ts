import { example, expect, expectAccessible, seed, test } from "./support";

test("a year's tax tables, the plan's by default and any other picked", async ({
  page,
}) => {
  await seed(
    page,
    { "/moving.toml": example("moving-states.toml") },
    "/moving.toml",
    "#/tools/tax-tables?year=2027",
  );
  await expect(
    page.getByRole("heading", { name: "Tax Tables", level: 1 }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Oregon income tax" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Income tax brackets" }),
  ).toBeVisible();
  await expect(
    page.getByRole("rowheader", { name: "Standard deduction" }).first(),
  ).toBeVisible();
  await expectAccessible(page);
  await page.getByLabel("Filing status").selectOption("single");
  await page.waitForURL(/status=single/);
  await page.getByLabel("State").selectOption("tx");
  await expect(
    page.getByRole("heading", { name: "Texas income tax" }),
  ).toBeVisible();
  await page.getByLabel("State").selectOption("");
  await expect.poll(() => page.url()).not.toContain("state=");
  await page.goto("#/tools/tax-tables?year=2060");
  await expect(
    page.getByRole("heading", { name: "Washington brackets" }),
  ).toBeVisible();
  await expect(page.getByText("This state has no income tax.")).toBeVisible();
});
