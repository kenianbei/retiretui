import {
  compareWith,
  example,
  expect,
  expectAccessible,
  isPhone,
  seed,
  test,
} from "./support";

const REPO = "https://github.com/kenianbei/retiretui";

test("the Start page starts a plan from an example or an upload", async ({
  page,
}) => {
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Start with a plan" }),
  ).toBeVisible();
  await expectAccessible(page);
  await page.getByRole("button", { name: /^Starter/ }).click();
  await expect(page.locator("header").getByText("starter.toml")).toBeVisible();
  await page.getByRole("button", { name: "File" }).click();
  await page.getByRole("menuitem", { name: "Upload a plan…" }).click();
  await page.locator('input[type=file][accept=".toml"]').setInputFiles({
    name: "early.toml",
    mimeType: "text/plain",
    buffer: Buffer.from(example("early-retiree.toml")),
  });
  await expect(page.locator("header").getByText("early.toml")).toBeVisible();
});

test("the app says what it is, and every page ends in the ways out of it", async ({
  page,
}, testInfo) => {
  await page.goto("./");
  await expect(
    page.getByText(/^RetireTui projects a household's finances/),
  ).toBeVisible();

  const footer = page.getByRole("contentinfo");
  await expect(footer).toContainText("not financial advice");
  await expect(footer).toContainText(/Version \d+\.\d+\.\d+/);
  await expect(
    footer.getByRole("link", { name: "Source on GitHub" }),
  ).toHaveAttribute("href", REPO);
  const report = await footer
    .getByRole("link", { name: "Report an issue" })
    .getAttribute("href");
  const [address, body] = (report ?? "").split("?body=");
  expect(address).toBe(`${REPO}/issues/new`);
  expect(decodeURIComponent(body ?? "")).toMatch(
    /\nRetireTui \d+\.\d+\.\d+, the web app\nMozilla\/.+$/,
  );

  const mark = page.getByRole("link", { name: "RetireTui on GitHub" });
  if (isPhone(testInfo)) {
    await expect(mark).toBeHidden();
  } else {
    await expect(mark).toHaveAttribute("href", REPO);
    await expect(mark).toHaveAttribute("target", "_blank");
  }

  await page.getByRole("button", { name: /^Starter/ }).click();
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();
  await expect(footer).toContainText("not financial advice");
});

test("an address that names no page says so", async ({ page }) => {
  await seed(
    page,
    { "/plan.toml": example("starter.toml") },
    "/plan.toml",
    "#/nowhere",
  );
  await expect(
    page.getByRole("heading", { name: "Page not found" }),
  ).toBeVisible();
  await page.goto("#/tools/nothing");
  await expect(
    page.getByText("There is no page at this address."),
  ).toBeVisible();
});

test("opening any plan but a compared one leaves nothing compared", async ({
  page,
}) => {
  await seed(
    page,
    {
      "/early.toml": example("early-retiree.toml"),
      "/couple.toml": example("retired-couple.toml"),
    },
    "/early.toml",
  );
  const compare = async () => {
    await page.goto("#/compare");
    await compareWith(page, "couple.toml");
  };
  const dropped = () =>
    page.waitForFunction(() => !location.hash.includes("with="));
  const file = () => page.getByRole("button", { name: "File" }).click();

  await compare();
  await file();
  await page.getByRole("menuitem", { name: "Add an example" }).click();
  await page.getByRole("menuitem", { name: /^Starter/ }).click();
  await expect(page.locator("header").getByText("starter.toml")).toBeVisible();
  await dropped();

  await compare();
  await page.locator('input[type=file][accept=".toml"]').setInputFiles({
    name: "uploaded.toml",
    mimeType: "text/plain",
    buffer: Buffer.from(example("retired-couple.toml")),
  });
  await expect(page.locator("header").getByText("uploaded.toml")).toBeVisible();
  await dropped();

  await compare();
  await file();
  await page.getByRole("menuitem", { name: "New plan…" }).click();
  await page.waitForURL(/#\/new\/household/);
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByLabel("Your name").fill("Jordan");
  await page.getByLabel("Birth year").fill("1975");
  await page.getByLabel("Salary").fill("90000");
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Create plan" }).click();
  await page.waitForURL(/#\/overview/);
  await expect(page.locator("header").getByText("Jordan.toml")).toBeVisible();
  await dropped();
});
