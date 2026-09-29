import { example, expect, expectAccessible, seed, test } from "./support";

/** A plan a line into which a key has lost its value, with one long line. */
const BROKEN = `schema = 1
# ${"a comment long enough to run past a phone's width ".repeat(3)}

[household]
filing
`;

test("a plan that does not parse is said at its line, its text shown", async ({
  page,
}) => {
  await seed(page, { "/broken.toml": BROKEN }, "/broken.toml");
  const alert = page.getByRole("alert");
  await expect(alert).toContainText("broken.toml could not be opened");
  await expect(alert).toContainText(
    "broken.toml, line 5, column 7: key with no value, expected `=`",
  );
  const written = page.getByRole("region", { name: "broken.toml as written" });
  await expect(written).toContainText("filing (the error)");
  const isWithinWidth = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth,
  );
  expect(isWithinWidth).toBe(true);
  await expectAccessible(page);

  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download broken.toml" }).click();
  expect((await download).suggestedFilename()).toBe("broken.toml");
});

test("a scenario whose base is gone is said in words, without text", async ({
  page,
}) => {
  const over = 'base = "gone.toml"\nschema = 1\n';
  await seed(page, { "/over.toml": over }, "/over.toml");
  await expect(page.getByRole("alert")).toContainText(
    "gone.toml could not be read",
  );
  await expect(page.getByRole("region", { name: /as written/ })).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Download over.toml" }),
  ).toBeVisible();
});

test("an amount too large to be likely needs attention, and is kept", async ({
  page,
}) => {
  const rich = example("starter.toml").replace(
    /balance = \d+/,
    "balance = 2000000000",
  );
  await seed(page, { "/rich.toml": rich }, "/rich.toml");
  const attention = page.getByRole("region", { name: "Needs attention" });
  const row = attention.getByRole("link", {
    name: /^Checking: a balance of \$2\.00B - check the amount/,
  });
  await expect(row).toBeVisible();
  await expect(page.getByRole("term")).toHaveCount(4);
  await row.click();
  await page.waitForURL(/#\/plan\/accounts/);
});
