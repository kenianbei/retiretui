import {
  SEARCH,
  example,
  expect,
  expectAccessible,
  isPhone,
  searchesDone,
  seed,
  test,
} from "./support";

test("ladders are searched, highlighted, taken and written as a scenario", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  await seed(
    page,
    { "/early.toml": example("early-retiree.toml") },
    "/early.toml",
    "#/overview?basis=nominal&year=2030",
  );
  await page.getByRole("link", { name: "Tools" }).first().click();
  await page.waitForURL(
    (url) =>
      url.hash.startsWith("#/tools/roth-conversions?") &&
      url.hash.includes("basis=nominal") &&
      url.hash.includes("year=2030"),
  );
  const constraints = page.getByRole("region", { name: "Constraints" });
  await expect(constraints.getByText("Convert to")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /^Conversions \(/ }),
  ).toBeVisible(SEARCH);
  await expectAccessible(page);
  const options = phone
    ? page.getByRole("region", { name: "Ladder options" }).getByRole("button")
    : page.getByRole("table", { name: "Ladder options" }).locator("tbody tr");
  await expect(options.nth(1)).toBeVisible();
  const second = phone ? options.nth(1) : options.nth(2);
  const label = (await second.innerText()).split(/\s/)[0] ?? "";
  await second.click();
  await expect(
    page.getByRole("heading", { name: `Conversions (${label})` }),
  ).toBeVisible();
  await page.waitForURL(new RegExp(`bracket=${label.replace("%", "")}`));
  await page.reload();
  await expect(
    page.getByRole("heading", { name: `Conversions (${label})` }),
  ).toBeVisible(SEARCH);

  await constraints.getByRole("link", { name: "Edit" }).click();
  const sheet = page.getByRole("dialog");
  await sheet.getByLabel("Fill bracket").fill("12");
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(sheet).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Conversions (12%)" }),
  ).toBeVisible(SEARCH);
  await searchesDone(page);
  await expect(options).toHaveCount(phone ? 1 : 2);
  const undo = page.getByRole("button", { name: "Undo" }).first();
  await expect(undo).toBeDisabled();

  await page.getByRole("button", { name: "Take this ladder" }).click();
  const asked = page.getByRole("alertdialog");
  await expect(
    asked.getByText(/^Take the 12% ladder\? \d+ conversion\(s\)/),
  ).toBeVisible();
  await asked.getByRole("button", { name: "Take", exact: true }).click();
  await expect(
    page.getByText(/took \d+ conversion\(s\) into the plan/),
  ).toBeVisible();
  await expect(undo).toBeEnabled();

  await searchesDone(page);
  await page.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(asked.getByText(/save first/)).toBeVisible();
  await asked.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await page.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(
    page.getByText("Wrote early-ladder-12.toml, compared."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open it" }).click();
  await expect(page.getByText("Scenario · read-only").first()).toBeAttached();
  await expect(
    page.getByRole("heading", { name: /^Conversions \(/ }),
  ).toBeVisible(SEARCH);
  await expect(
    page.getByRole("button", { name: "Take this ladder" }),
  ).toHaveCount(0);
});
