import {
  COMPACT_MONEY,
  FULL_MONEY,
  SIGNED_MONEY,
  SEARCH,
  example,
  expect,
  expectAccessible,
  isPhone,
  rowNamed,
  searchesDone,
  seed,
  test,
} from "./support";

const ROTH_MARCUS = `
[[accounts]]
id = "roth-ira-marcus"
name = "Marcus's Roth IRA"
kind = "ira"
roth = true
owner = "marcus"
balance = 20000
expected_return = 0.06
`;

test("each Roth owner's ladder and the household's claims are searched and taken", async ({
  page,
}, testInfo) => {
  const phone = isPhone(testInfo);
  const couple = example("mid-career-couple.toml").replace(
    "[[income]]",
    `${ROTH_MARCUS}\n[[income]]`,
  );
  await seed(page, { "/couple.toml": couple }, "/couple.toml");
  const undo = page.getByRole("button", { name: "Undo" }).first();

  const card = page.getByRole("region", { name: "Could do better" });
  await expect(
    card.getByRole("link", { name: /^Priya: (?!Searching)/ }),
  ).toBeVisible(SEARCH);
  await expect(
    card.getByRole("link", { name: /^Marcus: (?!Searching)/ }),
  ).toBeVisible(SEARCH);
  await card.getByRole("link", { name: /^Marcus: / }).click();
  await page.waitForURL(/#\/tools\/roth-conversions/);
  await expect(
    page
      .getByRole("region", { name: "Constraints" })
      .getByText("Marcus's Roth IRA"),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /^Conversions \(/ }),
  ).toBeVisible(SEARCH);

  await page.goto("#/tools/ssa-benefits");
  await expect(
    page.getByRole("heading", { name: "SSA Benefits" }),
  ).toBeVisible();
  await expect(
    page.getByText(/at full retirement age \(FRA, 66 to 67\)/),
  ).toBeVisible();
  const people = page.getByRole("region", { name: "People" });
  await expect(people.getByRole("group", { name: "Priya" })).toBeVisible();
  const shown = (text: string | RegExp) =>
    people.getByText(text, { exact: true }).filter({ visible: true }).first();
  await expect(shown("Stated")).toBeVisible();
  await expect(shown("No record")).toBeVisible();
  if (!phone)
    await expect(
      people.locator('th abbr[title="full retirement age"]'),
    ).toHaveText("FRA");
  const act = (name: string) =>
    people.getByRole("button", { name, exact: true });
  await act("Compute from record").click();
  await expect(
    people.getByText("Priya's benefit is computed from their record"),
  ).toBeVisible();
  await expect(shown("Computed")).toBeVisible();
  await expect(undo).toBeEnabled();
  const marcus = phone
    ? people.getByRole("button", { name: /^Marcus/ })
    : people.getByRole("row", { name: /Marcus/ });
  await marcus.click();
  await page.waitForURL(/person=1/);
  await expect(people.getByRole("group", { name: "Marcus" })).toBeVisible();
  await act("Compute from record").click();
  await expect(
    people.getByText("Marcus's benefit is computed from their record"),
  ).toBeVisible();
  const claims = page.getByRole("region", { name: "Claim options" });
  await expect(
    claims.getByRole("button", { name: "Take these claims" }),
  ).toBeVisible(SEARCH);
  await searchesDone(page);
  await expectAccessible(page);
  if (!phone) {
    await expect(claims.getByRole("table").locator("thead")).toContainText(
      /Priya[\s\S]*Marcus[\s\S]*Vs\. the plan/,
    );
    const options = claims.getByRole("table").locator("tbody tr");
    const [current, best] = [options.nth(0), options.nth(1)];
    await expect(current).not.toContainText(/[+-]\$/);
    await expect(best).toContainText(SIGNED_MONEY);
  }

  await page.goto("#/tools/ssa-benefits?person=0");
  await expect(people.getByRole("group", { name: "Priya" })).toBeVisible();
  await act("Hold claim").click();
  await page.waitForURL(/held=priya/);
  await expect(act("Let claim vary")).toBeVisible();
  await searchesDone(page);
  if (!phone) {
    await expect(claims.getByRole("table").locator("thead")).not.toContainText(
      "Priya",
    );
    await expect(people.getByRole("row", { name: /Priya/ })).toContainText(
      "Held",
    );
  }
  await page.getByRole("link", { name: "Overview" }).first().click();
  await page.waitForURL(/#\/overview\?.*held=priya/);
  await expect(
    card.getByRole("link", {
      name: /^(Claim Marcus at \d+|Claims as planned are best)/,
    }),
  ).toBeVisible(SEARCH);
  await page.goto("#/tools/ssa-benefits?person=0&held=priya");
  await act("Let claim vary").click();
  await page.waitForURL((url) => !url.hash.includes("held="));
  await searchesDone(page);

  await page.getByRole("button", { name: /^Save/ }).first().click();
  const options = phone
    ? claims.getByRole("button", { name: /·/ })
    : claims.getByRole("table").locator("tbody tr");
  await (phone ? options.nth(1) : options.nth(2)).click();
  await page.waitForURL(/claim=\d+-\d+/);
  const key = /claim=(\d+-\d+)/.exec(page.url())?.[1] ?? "";
  await page.reload();
  await expect(
    claims.getByRole("button", { name: "Take these claims" }),
  ).toBeVisible(SEARCH);
  expect(page.url()).toContain(`claim=${key}`);
  const [priya, marcusAge] = key.split("-");
  await searchesDone(page);
  await claims.getByRole("button", { name: "Take these claims" }).click();
  const asked = page.getByRole("alertdialog");
  await expect(
    asked.getByText(
      `Take these claims? Priya at ${priya ?? ""}, Marcus at ${marcusAge ?? ""}.`,
    ),
  ).toBeVisible();
  await asked.getByRole("button", { name: "Take", exact: true }).click();
  await expect(
    page.getByText(
      `claimed Priya at ${priya ?? ""}, Marcus at ${marcusAge ?? ""}`,
    ),
  ).toBeVisible();
  await searchesDone(page);
  await claims.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(asked.getByText(/save first/)).toBeVisible();
  await asked.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: /^Save/ }).first().click();

  await expect(people.getByRole("group", { name: "Priya" })).toBeVisible();
  await act("Estimate from salary").click();
  await expect(
    people.getByText(/estimated \d+ years of earnings for Priya/),
  ).toBeVisible();
  await act("Clear record…").click();
  await expect(asked.getByText("Clear Priya's earnings record?")).toBeVisible();
  await asked.getByRole("button", { name: "Clear" }).click();
  await expect(
    people.getByText("cleared Priya's earnings record"),
  ).toBeVisible();
  await act("Remove Social Security…").click();
  await asked.getByRole("button", { name: "Remove" }).click();
  await expect(
    people.getByText("removed Priya's Social Security income"),
  ).toBeVisible();
  await undo.click();
  await undo.click();
  await undo.click();
  await page.getByRole("button", { name: /^Save/ }).first().click();
  await searchesDone(page);
  await claims.getByRole("button", { name: "Write as a scenario" }).click();
  await asked.getByRole("button", { name: "Write" }).click();
  await expect(
    page.getByText(/^Wrote couple-claims-\d+-\d+\.toml, compared\./),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open it" }).click();
  await expect(page.getByText("Scenario · read-only").first()).toBeAttached();
  await expect(
    claims.getByRole("button", { name: "Write as a scenario" }),
  ).toBeVisible(SEARCH);
  await expect(
    claims.getByRole("button", { name: "Take these claims" }),
  ).toHaveCount(0);
});

test("a monthly benefit and what a claim does against the plan are said in full", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/robin.toml": example("with-earnings.toml") },
    "/robin.toml",
    "#/tools/ssa-benefits",
  );
  await expect(rowNamed(page, testInfo, "Robin")).toContainText(FULL_MONEY);
  const claims = page.getByRole("region", { name: "Claim options" });
  await expect(
    claims.getByRole("button", { name: "Take these claims" }),
  ).toBeVisible(SEARCH);
  await expect(claims).toContainText(SIGNED_MONEY);
  await expect(claims).not.toContainText(COMPACT_MONEY);
});
