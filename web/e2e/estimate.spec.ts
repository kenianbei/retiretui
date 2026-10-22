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
    const rows = proposals.getByRole("table").locator("tbody tr");
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText("Groceries");
    await expect(rows.nth(1)).toContainText("Utilities");
  }
  await rowNamed(page, testInfo, "Utilities").click();
  await page.waitForURL(/row=1/);
  const readOut = page.getByRole("region", { name: "Proposal", exact: true });
  await expect(readOut).toContainText("CITY OF SAN JOSE WATER");
  await readOut.getByRole("link", { name: "Edit" }).click();
  await page.waitForURL(/edit=true/);
  const sheet = page.getByRole("dialog");
  const amount = sheet.getByLabel("Amount", { exact: true });
  await amount.focus();
  await amount.fill("1200");
  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(readOut).toContainText("$1,200 a year");

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
