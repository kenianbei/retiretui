import { expect, test } from "@playwright/test";

// The suite's own `test` fails on what a page logs as an error, which a refusal is.

const POLICY = "Content-Security-Policy";

test("a script written into the page is refused", async ({ page }) => {
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Start with a plan" }),
  ).toBeVisible();
  const refused = await page.evaluate(
    () =>
      new Promise<string>((resolve) => {
        document.addEventListener("securitypolicyviolation", (event) => {
          resolve(event.violatedDirective);
        });
        const script = document.createElement("script");
        script.textContent = "window.injected = true";
        document.body.append(script);
      }),
  );
  expect(refused).toMatch(/^script-src/);
  expect(await page.evaluate(() => "injected" in window)).toBe(false);
});

test("the policy stands first in the page, and admits a script written into it by hash alone", async ({
  page,
}) => {
  await page.goto("./");
  const first = page.locator("head > :first-child");
  await expect(first).toHaveAttribute("http-equiv", POLICY);
  const policy = (await first.getAttribute("content")) ?? "";
  const scripts = policy
    .split("; ")
    .find((directive) => directive.startsWith("script-src "));
  expect(scripts).toMatch(/ 'sha256-[\w+/]+=*'$/);
  expect(scripts).not.toContain("'unsafe-inline'");
});
