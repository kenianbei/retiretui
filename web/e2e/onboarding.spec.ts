import { expect, expectAccessible, test } from "./support";

const ANSWERS = "retiretui-app:new-plan";

test("a first plan is made from a few questions, read back to be changed", async ({
  page,
}) => {
  await page.goto("./");
  await page.getByRole("link", { name: "Answer a few questions" }).click();
  await page.waitForURL(/#\/new\/household/);
  await expect(
    page.getByRole("heading", { name: "Your household" }),
  ).toBeVisible();
  await expectAccessible(page);
  await page.getByLabel("Filing status").selectOption({ label: "Single" });
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL(/#\/new\/you/);
  await page.getByLabel("Your name").fill("Jordan");
  await page.getByLabel("Birth year").fill("1975");
  await page.getByLabel("Salary").fill("90000");
  await page.getByLabel("Salary").blur();
  await page.reload();
  await expect(page.getByLabel("Your name")).toHaveValue("Jordan");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL(/#\/new\/check/);
  await expect(
    page.getByRole("heading", { name: "Check your answers" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "About your partner" }),
  ).toHaveCount(0);
  await expectAccessible(page);

  await page.getByRole("link", { name: "Change Birth year" }).click();
  await page.waitForURL(/#\/new\/you\?.*field=birth_year/);
  await expect(page.getByLabel("Birth year")).toBeFocused();
  await page.getByLabel("Birth year").fill("1976");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL(/#\/new\/check/);
  await expect(page.getByText("1976", { exact: true })).toBeVisible();

  await page.getByRole("link", { name: "Change Filing status" }).click();
  await page.getByLabel("Filing status").selectOption("married-joint");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL(/#\/new\/check/);
  await expect(
    page.getByRole("heading", { name: "About your partner" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Change Partner's name" }).click();
  await page.getByLabel("Partner's name").fill("Alex");
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByText("Alex", { exact: true })).toBeVisible();

  await expect(page.getByLabel("File name")).toHaveValue("Jordan");
  await page.getByRole("button", { name: "Create plan" }).click();
  await page.waitForURL(/#\/overview/);
  await expect(page.locator("header").getByText("Jordan.toml")).toBeVisible();
  expect(
    await page.evaluate((key) => sessionStorage.getItem(key), ANSWERS),
  ).toBeNull();

  await page.getByRole("button", { name: "File" }).click();
  await page.getByRole("menuitem", { name: "New plan…" }).click();
  await page.waitForURL(/#\/new\/household/);
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByLabel("Your name")).toHaveValue("");
  await page.getByLabel("Your name").fill("Jordan");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.waitForURL(/#\/new\/check/);
  await page.getByRole("button", { name: "Create plan" }).click();
  await expect(
    page.getByRole("alertdialog").getByText("Replace Jordan.toml?"),
  ).toBeVisible();
  await page.getByRole("button", { name: "Keep the one I have" }).click();
  await expect(
    page.getByRole("heading", { name: "Check your answers" }),
  ).toBeVisible();
  await page.getByLabel("File name").fill("Second");
  await page.getByRole("button", { name: "Create plan" }).click();
  await page.waitForURL(/#\/overview/);
  await expect(page.locator("header").getByText("Second.toml")).toBeVisible();

  await page.goto("#/new/you");
  await page.getByLabel("Your name").fill("Temp");
  await page.getByRole("button", { name: "Cancel" }).click();
  await page.waitForURL(/#\/overview/);
  expect(
    await page.evaluate((key) => sessionStorage.getItem(key), ANSWERS),
  ).toBeNull();

  await page.goto("#/plan/people?edit=0");
  await page.getByLabel("Name", { exact: true }).fill("Renamed");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.goto("#/new/household");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByLabel("File name").fill("Third");
  await page.getByRole("button", { name: "Create plan" }).click();
  await expect(page.getByText("Save your edits first?")).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).last().click();
  expect(page.url()).toMatch(/#\/new\/check/);
});
