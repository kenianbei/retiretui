import {
  compareWith,
  example,
  expect,
  expectAccessible,
  seed,
  test,
} from "./support";

test("plans are renamed, their scenarios following, and deleted", async ({
  page,
  context,
}) => {
  await seed(
    page,
    {
      "/early.toml": example("early-retiree.toml"),
      "/couple.toml": example("retired-couple.toml"),
      "/ladder.toml":
        'schema = 1\nbase = "early.toml"\n\n[plan]\ninflation = 0.03\n',
    },
    "/early.toml",
    "#/compare",
  );
  await compareWith(page, "couple.toml");

  const manage = async () => {
    await page.getByRole("button", { name: "File" }).click();
    await page.getByRole("menuitem", { name: "Manage plans…" }).click();
    await expect(page.locator("[data-slot=dropdown-menu-content]")).toHaveCount(
      0,
    );
    return page.getByRole("dialog", { name: "Manage plans" });
  };
  const close = async (dialog: ReturnType<typeof page.getByRole>) => {
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  };
  const rename = async (from: string, to: string) => {
    const dialog = await manage();
    await dialog.getByRole("button", { name: `Rename ${from}` }).click();
    const ask = page.getByRole("alertdialog");
    await ask.getByLabel("File name").fill(to);
    await ask.getByRole("button", { name: "Rename" }).click();
    await expect(ask).toHaveCount(0);
    await close(dialog);
  };
  const stored = (path: string) =>
    page.evaluate(
      (path) => localStorage.getItem(`retiretui-app:file:${path}`),
      path,
    );

  const dialog = await manage();
  await expect(dialog.getByText("Scenario of early.toml")).toBeVisible();
  await expectAccessible(page);
  await dialog.getByRole("button", { name: "Rename couple.toml" }).click();
  const ask = page.getByRole("alertdialog");
  await ask.getByLabel("File name").fill("early");
  await expect(
    ask.getByText("already has a plan named early.toml"),
  ).toBeVisible();
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(ask).toHaveCount(0);
  await close(dialog);

  await rename("couple.toml", "pair.toml");
  await page.waitForURL(/pair\.toml/);
  expect(page.url()).not.toContain("couple");

  await page.goto("#/plan/settings?edit=0");
  await page
    .getByRole("dialog")
    .getByLabel("Inflation", { exact: true })
    .fill("4.5");
  await page.getByRole("dialog").getByRole("button", { name: "Apply" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await rename("early.toml", "mine.toml");
  await expect(page.locator("header").getByText("mine.toml")).toBeVisible();
  expect(await stored("/early.toml")).toBeNull();
  expect(await stored("/ladder.toml")).toMatch(/base = "mine\.toml"/);
  await expect(
    page.getByRole("button", { name: /unsaved edits/ }),
  ).toBeEnabled();

  const other = await context.newPage();
  await other.goto("#/overview");
  await expect(other.locator("header").getByText("mine.toml")).toBeVisible();
  await page.getByRole("button", { name: /^Save/ }).click();
  await rename("mine.toml", "ours.toml");
  await expect(other.locator("header").getByText("ours.toml")).toBeVisible();

  const listed = await manage();
  await listed.getByRole("button", { name: "Delete ours.toml" }).click();
  const question = page.getByRole("alertdialog");
  await expect(question.getByText("ladder.toml")).toBeVisible();
  await question.getByRole("button", { name: "Delete" }).click();
  await expect(question).toHaveCount(0);
  await close(listed);
  await expect(
    page.getByRole("heading", { name: "Start with a plan" }),
  ).toBeVisible();
  expect(await stored("/ours.toml")).toBeNull();
  await expect(
    page.getByText("Your plans are kept only in this browser"),
  ).toBeVisible();
});
