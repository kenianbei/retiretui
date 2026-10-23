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
  await expectAccessible(page);
  await records
    .getByRole("button", { name: /^Ignore / })
    .first()
    .click();
  await expect(page.getByText(/1 record ignored/)).toBeVisible();
  await rowNamed(page, testInfo, "Ignored").click();
  const ignored = page.getByRole("table", { name: "Records: ignored" });
  await ignored.getByRole("button", { name: /^Count / }).click();
  await expect(page.getByText(/record ignored/)).toHaveCount(0);

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
