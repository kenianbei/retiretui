import {
  example,
  expect,
  expectAccessible,
  isPhone,
  seed,
  test,
} from "./support";

test("the Ledger tables every year over the chosen year's flows", async ({
  page,
}, testInfo) => {
  await seed(
    page,
    { "/starter.toml": example("starter.toml") },
    "/starter.toml",
    "#/ledger",
  );
  const table = page.getByRole("table", { name: /year by year/ });
  const rows = table.locator("tbody tr");
  await expect(rows.nth(20)).toBeVisible();
  const flows = page.getByRole("table", { name: /Flows/ });
  await expect(flows).toBeVisible();
  const isFlowsFirst = await flows.evaluate(
    (element, years) =>
      Boolean(
        element.compareDocumentPosition(years) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ),
    await table.elementHandle(),
  );
  expect(isFlowsFirst, "a phone reads the year's flows first").toBe(
    isPhone(testInfo),
  );
  const target = rows.nth(5);
  const year = (
    (await target.locator("td").first().textContent()) ?? ""
  ).trim();
  await target.click();
  await page.waitForURL(new RegExp(`year=${year}`));
  await expect(target).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText(`${year} Flows`)).toBeVisible();
  await expect(page.getByText(`${year} Income & Tax`)).toBeVisible();
  await page.keyboard.press("ArrowRight");
  const next = String(Number(year) + 1);
  await expect(page.getByText(`${next} Flows`)).toBeVisible();
  await expectAccessible(page);

  const scroller = table.locator("xpath=..");
  await scroller.evaluate((element) => {
    element.scrollLeft = 400;
  });
  const pinned = await table.locator("tbody tr td").first().boundingBox();
  const frame = await scroller.boundingBox();
  expect(pinned?.x ?? 0).toBeLessThanOrEqual((frame?.x ?? 0) + 2);

  const today = await rows.nth(5).locator("td").last().textContent();
  await page.getByRole("link", { name: "future dollars" }).click();
  await page.waitForURL(/basis=nominal/);
  await expect(rows.nth(5).locator("td").last()).not.toHaveText(today ?? "");
  await page.reload();
  await expect(page.getByText(`${next} Flows · future dollars`)).toBeVisible();
});
