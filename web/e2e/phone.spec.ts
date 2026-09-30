import type { Locator, Page } from "@playwright/test";

import {
  example,
  expect,
  expectAccessible,
  managePlans,
  openPlan,
  searchesDone,
  seed,
  test,
} from "./support";

/** The page is no wider than the screen it is drawn on. */
async function expectFits(page: Page, hash: string) {
  const [scrolled, shown] = await page.evaluate(() => [
    document.documentElement.scrollWidth,
    document.documentElement.clientWidth,
  ]);
  expect(scrolled, `${hash} scrolls sideways`).toBeLessThanOrEqual(shown);
}

/** The pages a grouped tab's chips lead to, from one of them. */
async function pagesOf(
  page: Page,
  group: string,
  first: string,
): Promise<string[]> {
  await page.goto(first);
  const links = page.getByRole("navigation", { name: group }).getByRole("link");
  await expect(links.first()).toBeVisible();
  return links.evaluateAll((each) =>
    each.map((link) => link.getAttribute("href") ?? ""),
  );
}

test.skip(({ isMobile }) => !isMobile, "the widths it holds to are a phone's");

test("every page fits a phone's width", async ({ page }) => {
  test.setTimeout(300_000);
  await seed(
    page,
    { "/couple.toml": example("mid-career-couple.toml") },
    "/couple.toml",
  );
  const hashes = [
    "#/overview",
    "#/ledger",
    "#/compare",
    ...(await pagesOf(page, "Tools", "#/tools/roth-conversions")),
    ...(await pagesOf(page, "Plan", "#/plan/accounts")),
  ];
  expect(hashes.length, "the Tools and Plan pages were found").toBeGreaterThan(
    15,
  );
  for (const hash of hashes) {
    await page.goto(hash.startsWith("#") ? hash : `#${hash}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await searchesDone(page);
    await expectFits(page, hash);
  }
});

/** The header's file name shows whole, not cut short. */
async function expectNameWhole(page: Page, name: string) {
  const shown = page.getByRole("banner").getByText(name, { exact: true });
  await expect(shown).toBeVisible();
  const isCut = await shown.evaluate(
    (element) => element.scrollWidth > element.clientWidth,
  );
  expect(isCut, `${name} is cut short`).toBe(false);
}

test("the header names the file with an issue or a scenario open", async ({
  page,
}) => {
  await seed(
    page,
    {
      "/short.toml": example("starter.toml").replace(
        "balance = 5000",
        "balance = -1",
      ),
      "/early.toml": example("early-retiree.toml"),
      "/what-if.toml": 'schema = 1\nbase = "early.toml"\n',
    },
    "/short.toml",
  );
  await expect(
    page.getByRole("button", { name: /^\d+ issues?$/ }),
  ).toBeVisible();
  await expectNameWhole(page, "short.toml");

  await openPlan(page, "what-if.toml");
  await expectNameWhole(page, "what-if.toml");
  await page.getByRole("button", { name: /^File, what-if\.toml/ }).click();
  await expect(
    page.getByRole("menu").getByText("Scenario · read-only"),
  ).toBeVisible();
  await page.keyboard.press("Escape");
});

test("the palette opens by touch, and a group's chips reach their page", async ({
  page,
}) => {
  await seed(
    page,
    { "/couple.toml": example("mid-career-couple.toml") },
    "/couple.toml",
    "#/plan/market",
  );
  const chips = page.getByRole("navigation", { name: "Plan" });
  const current = chips.getByRole("link", { name: "Market" });
  await expect(current).toBeInViewport({ ratio: 0.95 });
  const hit = await current.evaluate((chip) => {
    const drawn = chip.getBoundingClientRect();
    const above = document.elementFromPoint(
      drawn.left + drawn.width / 2,
      drawn.top - 6,
    );
    return above?.textContent ?? "";
  });
  expect(hit, "a chip is touched above its drawn edge").toBe("Market");

  await page
    .getByRole("button", { name: "Find a page, a plan or an action" })
    .tap();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expectFits(page, "the palette");
  await expectAccessible(page);
});

/** Every one of `items` is centred on one line. */
async function expectOneRow(items: Locator, what: string) {
  const middles = await items.evaluateAll((each) =>
    each.map((item) => {
      const box = item.getBoundingClientRect();
      return Math.round(box.top + box.height / 2);
    }),
  );
  expect(middles.length, `${what} are there`).toBeGreaterThan(1);
  expect(new Set(middles).size, `${what} wrap`).toBe(1);
}

test("a long name keeps its row's actions beside it", async ({ page }) => {
  const long = "a-plan-named-at-length-for-the-early-retirement-scenario.toml";
  await seed(
    page,
    {
      "/couple.toml": example("mid-career-couple.toml"),
      [`/${long}`]: example("starter.toml"),
    },
    "/couple.toml",
  );
  const row = (await managePlans(page))
    .getByRole("listitem")
    .filter({ hasText: long });
  await expectOneRow(
    row.getByRole("button").or(row.getByText(long, { exact: true })),
    "a long name's actions",
  );
});
