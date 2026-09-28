import { readFileSync } from "node:fs";

import AxeBuilder from "@axe-core/playwright";
import {
  test as base,
  expect,
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
