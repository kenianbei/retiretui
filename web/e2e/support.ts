import { readFileSync } from "node:fs";

import AxeBuilder from "@axe-core/playwright";
import {
  test as base,
  expect,
  type Locator,
  type Page,
  type TestInfo,
} from "@playwright/test";

/** The WCAG levels every page is held to. */
const WCAG = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"];

const EXAMPLES = new URL(
  "../../crates/retiretui_client/src/setup/examples/",
  import.meta.url,
);

/** An example plan's text, as the client ships it. */
export function example(file: string): string {
  return readFileSync(new URL(file, EXAMPLES), "utf8");
}

/** A Social Security statement as ssa.gov gives one. */
export const STATEMENT = new URL(
  "../../crates/retiretui_engine/tests/fixtures/statement.xml",
  import.meta.url,
).pathname;

/**
 * Opens the app at `hash` over a workspace of `files`, `last` the document
 * open: written before the page's first script runs, once for the context,
 * so that a reload or a second tab finds what the test left.
 */
export async function seed(
  page: Page,
  files: Record<string, string>,
  last: string | null,
  hash = "#/overview",
) {
  await page.context().addInitScript(
    ([files, last]) => {
      const SEEDED = "e2e:seeded";
      if (localStorage.getItem(SEEDED) !== null) return;
      localStorage.setItem(SEEDED, "");
      for (const [path, text] of Object.entries(files)) {
        localStorage.setItem(`retiretui-app:file:${path}`, text);
      }
      if (last) localStorage.setItem("retiretui-app:last", last);
    },
    [files, last] as const,
  );
  await page.goto(hash);
}

/** How long a search in a worker may take, on the slowest runner. */
export const SEARCH = { timeout: 110_000 };

/** Whether the project is a phone's. */
export function isPhone(testInfo: TestInfo): boolean {
  return testInfo.project.name.endsWith("-phone");
}

/** A row by what its name begins with: a table's, or on a phone its list's button. */
export function rowNamed(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Locator {
  const named = new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`);
  return isPhone(testInfo)
    ? page.getByRole("button", { name: named })
    : page.getByRole("row", { name: named });
}

/** Every search on the page has answered. */
export async function searchesDone(page: Page) {
  await expect(page.getByText("Searching…")).toHaveCount(0, SEARCH);
}

/** Opens the workspace's `name` from the File menu. */
export async function openPlan(page: Page, name: string) {
  await page.getByRole("button", { name: "File" }).click();
  await page.getByRole("menuitem", { name: "Open" }).click();
  await page.getByRole("menuitem", { name }).click();
}

/** Ticks the workspace's `name` in among the plans compared. */
export async function compareWith(page: Page, name: string) {
  await page.getByRole("button", { name: "Compare with" }).click();
  await page.getByRole("menuitemcheckbox", { name }).click();
  await page.keyboard.press("Escape");
  await page.waitForURL(/with=/);
}

/** The page as it stands breaks no WCAG A or AA rule axe can check. */
export async function expectAccessible(page: Page) {
  const { violations } = await new AxeBuilder({ page })
    .withTags(WCAG)
    .analyze();
  const said = violations.map(
    (violation) =>
      `${violation.id}: ${violation.nodes.map((node) => node.target.join(" ")).join(", ")}`,
  );
  expect(said).toEqual([]);
}

/** Every test, failing on anything the page throws or logs as an error. */
export const test = base.extend<{ pageErrors: undefined }>({
  pageErrors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => {
        if (message.type() === "error") errors.push(message.text());
      });
      await use(undefined);
      expect(errors).toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };
