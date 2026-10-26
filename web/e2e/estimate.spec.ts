import {
  example,
  expect,
  expectAccessible,
  isPhone,
  rowNamed,
  seed,
  test,
} from "./support";

const RETIRED = example("retired-couple.toml");
const CHASE = new URL(
  "../../crates/retiretui_engine/tests/fixtures/bank/chase.csv",
  import.meta.url,
).pathname;
const OFX = new URL(
  "../../crates/retiretui_engine/tests/fixtures/bank/checking.ofx",
  import.meta.url,
).pathname;

test("exports are read, a proposal edited, and the proposals taken and written", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/retired.toml": RETIRED },
    "/retired.toml",
    "#/tools/spending-estimate",
  );
  await expect(
    page.getByRole("heading", { name: "Spending Estimate" }),
  ).toBeVisible();
  await expect(
    page.getByText(/Add a bank's CSV, OFX or QFX export/),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: /^Take/ })).toHaveCount(0);

  const picker = page.getByLabel("Bank export");
  await picker.setInputFiles(CHASE);
  const files = page.getByRole("region", { name: "Files" });
  await expect(files.getByText("chase.csv")).toBeVisible();
  await expect(files.getByText("5 rows")).toBeVisible();
  await picker.setInputFiles(OFX);
  await expect(files.getByText("checking.ofx")).toBeVisible();
  await expect(page.getByText(/^1 month through Oct 2026/)).toBeVisible();
  await expectAccessible(page);

  const proposals = page.getByRole("region", { name: "Proposals" });
  if (!isPhone(testInfo)) {
    const rows = proposals
      .getByRole("table", { name: "Proposals" })
      .locator("tbody tr");
    await expect(rows).toHaveCount(4);
    await expect(rows.nth(0)).toContainText("Groceries (new)");
    await expect(rows.nth(1)).toContainText("Groceries");
    await expect(rows.nth(2)).toContainText("Utilities (new)");
  }
  await rowNamed(page, testInfo, "Utilities (new)").click();
  await page.waitForURL(/row=2/);
  const readOut = page.getByRole("region", { name: "Expense", exact: true });
  await expect(
    page.getByRole("table", { name: "Payees: Utilities" }),
  ).toContainText("CITY OF SAN JOSE WATER");
  await readOut.getByRole("link", { name: "Edit" }).click();
  await page.waitForURL(/edit=row/);
  const sheet = page.getByRole("dialog");
  const amount = sheet.getByLabel("Amount", { exact: true });
  await amount.focus();
  await amount.fill("1200");
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(readOut).toContainText("$1,200 a year");

  await rowNamed(page, testInfo, "Utilities").last().click();
  await page.waitForURL(/row=3/);
  const records = page.getByRole("table", { name: "Records: Utilities" });
  await expect(records).toContainText("2026-10-");
  await expect(records).toContainText("$");
  const menuOf = async (table: typeof records) => {
    await table
      .getByRole("button", { name: /^Actions for / })
      .first()
      .click();
    return page.getByRole("menu");
  };
  // A pointer sent straight from the trigger into the submenu reads as leaving
  // the menu, which closes it under the click, so the pick is dispatched.
  const pickCategory = (name: string | RegExp) =>
    page.getByRole("menuitem", { name }).dispatchEvent("click");
  await (await menuOf(records))
    .getByRole("menuitem", { name: "Ignore this record" })
    .click();
  await expect(page.getByText(/1 record ignored/)).toBeVisible();
  await expect(page.getByRole("status")).toContainText("is ignored");
  await rowNamed(page, testInfo, "Ignored").click();
  const ignored = page.getByRole("table", { name: "Records: ignored" });
  await (await menuOf(ignored))
    .getByRole("menuitem", { name: "Count this record again" })
    .click();
  await expect(page.getByText(/record ignored/)).toHaveCount(0);

  await rowNamed(page, testInfo, "Utilities").last().click();
  const menu = await menuOf(records);
  await menu.getByRole("menuitem", { name: /is…$/ }).click();
  await pickCategory("18 Pets");
  await expect(page.getByRole("status")).toContainText("is Pets");
  await rowNamed(page, testInfo, "Pets").last().click();
  const toBucket = await menuOf(
    page.getByRole("table", { name: "Records: Pets" }),
  );
  await toBucket.getByRole("menuitem", { name: /is…$/ }).click();
  await pickCategory(/^0 /);
  await expect(page.getByRole("status")).toContainText("goes to the bucket");
  await rowNamed(page, testInfo, "CITY OF SAN JOSE WATER").click();
  const unnamed = page.getByRole("table", {
    name: "Records: CITY OF SAN JOSE WATER",
  });
  const back = await menuOf(unnamed);
  await back.getByRole("menuitem", { name: /is…$/ }).click();
  await pickCategory("18 Pets");
  await expect(page.getByRole("status")).toContainText("is Pets");
  await expectAccessible(page);
  await rowNamed(page, testInfo, "Pets").last().click();
  const pets = page.getByRole("table", { name: "Records: Pets" });
  await (await menuOf(pets))
    .getByRole("menuitem", { name: "Payees containing…" })
    .click();
  const words = page.getByRole("alertdialog");
  await expect(words.getByRole("status")).toContainText("a year");
  await words.getByLabel("Category").selectOption({ label: "12 Internet" });
  await words.getByRole("button", { name: "Apply" }).click();
  await expect(page.getByRole("status")).toContainText("are Internet");

  await proposals.getByRole("button", { name: "Write as a scenario" }).click();
  const asked = page.getByRole("alertdialog");
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(
    page.getByText(/^Wrote retired-estimate\.toml, compared\./),
  ).toBeVisible();

  await proposals.getByRole("button", { name: "Take these proposals" }).click();
  await expect(
    asked.getByText(/^Set 0 expenses and add 2 from 1 month/),
  ).toBeVisible();
  await asked.getByRole("button", { name: "Take", exact: true }).click();
  await expect(page.getByText(/^0 expenses set and 2 added/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Undo" }).first(),
  ).toBeEnabled();

  await files.getByRole("button", { name: "Remove chase.csv" }).click();
  await expect(files.getByText("chase.csv")).toHaveCount(0);
  await files.getByRole("button", { name: "Remove checking.ofx" }).click();
  await expect(
    page.getByText(/Add a bank's CSV, OFX or QFX export/),
  ).toBeVisible();
});

test("an export that does not read is said", async ({ page }) => {
  await seed(
    page,
    { "/retired.toml": RETIRED },
    "/retired.toml",
    "#/tools/spending-estimate",
  );
  await page.getByLabel("Bank export").setInputFiles({
    name: "notes.csv",
    mimeType: "text/csv",
    buffer: Buffer.from("just a line\n"),
  });
  await expect(page.getByRole("alert")).toContainText("notes.csv");
});

test("the plan's estimate settings fold categories into its expenses", async ({
  page,
}) => {
  await seed(
    page,
    { "/retired.toml": RETIRED },
    "/retired.toml",
    "#/tools/spending-estimate",
  );
  const picker = page.getByLabel("Bank export");
  await picker.setInputFiles(CHASE);
  await picker.setInputFiles(OFX);
  const proposals = page.getByRole("region", { name: "Proposals" });
  await expect(
    proposals.getByText("Utilities (new)").filter({ visible: true }),
  ).toBeVisible();

  const settings = page.getByRole("region", { name: "Settings" });
  await expect(settings).toContainText("New ones for large categories");
  await settings.getByRole("link", { name: "Edit" }).click();
  await page.waitForURL(/edit=settings/);
  const sheet = page.getByRole("dialog");
  await sheet
    .getByLabel("Lean", { exact: true })
    .selectOption({ label: "Existing expenses only" });
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(settings).toContainText("Existing expenses only");
  await expect(
    proposals.getByText("Housing").filter({ visible: true }).first(),
  ).toBeVisible();
  await expect(
    proposals.getByText("Other (new bucket)").filter({ visible: true }),
  ).toBeVisible();
  await expect(proposals.getByText("Utilities (new)")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Undo" }).first(),
  ).toBeEnabled();
  await expectAccessible(page);
});
