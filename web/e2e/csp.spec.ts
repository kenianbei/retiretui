import { expect, test } from "@playwright/test";

// The suite's own `test` fails on what a page logs as an error, which a refusal is.

test("a script written into the page is refused", async ({ page }) => {
  await page.goto("./");
  const refused = await page.evaluate(
    () =>
      new Promise<string>((resolve) => {
        document.addEventListener("securitypolicyviolation", (event) => {
          resolve(event.violatedDirective);
        });
        const script = document.createElement("script");
        script.textContent = "window.injected = true";
        document.body.append(script);
        if ("injected" in window) resolve("the script ran");
      }),
  );
  expect(refused).toMatch(/^script-src/);
});

test("the policy stands first in the page", async ({ page }) => {
  await page.goto("./");
  await expect(page.locator("head > :first-child")).toHaveAttribute(
    "http-equiv",
    "Content-Security-Policy",
  );
});
