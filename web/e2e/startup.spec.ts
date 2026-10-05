import { expect, type Page, test } from "@playwright/test";

// The suite's own `test` fails on what a page throws, which is what these are about.
// A worker in control would answer requests the tests mean to refuse.
test.use({ serviceWorkers: "block" });

const failure = (page: Page) => page.getByRole("alert");

/** The page says `heading` and `reason` in the app's place, and a report opens with the browser's name. */
async function expectFailed(
  page: Page,
  reason: string | RegExp,
  heading = "RetireTui did not start",
) {
  await expect(page.getByRole("heading", { name: heading })).toBeVisible();
  await expect(failure(page)).toContainText(reason);
  await expect(page.getByText("Loading RetireTui…")).toBeHidden();
  const report = await failure(page)
    .getByRole("link", { name: "report an issue" })
    .getAttribute("href");
  expect(decodeURIComponent(report ?? "")).toMatch(
    /\/retiretui\/issues\/new\?body=[\s\S]+\nMozilla\//,
  );
}

test.describe("with JavaScript off", () => {
  test.use({ javaScriptEnabled: false });

  test("the page says what it needs", async ({ page }) => {
    await page.goto("./");
    await expect(
      page.getByRole("heading", { name: "RetireTui needs JavaScript" }),
    ).toBeVisible();
    await expect(page.getByText("Loading RetireTui…")).toBeHidden();
  });
});

test("the loading line gives way to the app, which nothing later replaces", async ({
  page,
}) => {
  await page.goto("./");
  const start = page.getByRole("heading", { name: "Start with a plan" });
  await expect(start).toBeVisible();
  await expect(page.getByText("Loading RetireTui…")).toHaveCount(0);
  await page.evaluate(() => {
    void Promise.reject(new Error("a search gone wrong"));
  });
  await page.evaluate(() => new Promise(requestAnimationFrame));
  await expect(start).toBeVisible();
  await expect(failure(page)).toBeHidden();
});

test("an error that is not the app's does not keep the app from showing", async ({
  page,
}) => {
  // Thrown once the page listens, and before the app's script has run.
  await page.addInitScript(() => {
    let startup: Window["startup"];
    Object.defineProperty(window, "startup", {
      get: () => startup,
      set(value: Window["startup"]) {
        startup = value;
        setTimeout(() => {
          throw new Error("an extension's own");
        });
      },
    });
  });
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Start with a plan" }),
  ).toBeVisible();
  await expect(failure(page)).toBeHidden();
});

test("a browser without WebAssembly is told so", async ({ page }) => {
  await page.addInitScript(() => {
    Reflect.deleteProperty(globalThis, "WebAssembly");
  });
  await page.goto("./");
  await expectFailed(page, "does not offer WebAssembly");
});

test("a browser that blocks the page's storage is told so", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      get() {
        throw new DOMException("denied", "SecurityError");
      },
    });
  });
  await page.goto("./");
  await expectFailed(page, "Allow cookies and site data");
});

test("the bindings refused are said as the browser says it", async ({
  page,
}) => {
  await page.route("**/*.wasm", (route) => route.abort());
  await page.goto("./");
  await expectFailed(page, "The browser says: TypeError");
});

test("a script the browser cannot parse is said, not left blank", async ({
  page,
}) => {
  await page.route("**/assets/index-*.js", (route) =>
    route.fulfill({ contentType: "text/javascript", body: "await = ;" }),
  );
  await page.goto("./");
  await expectFailed(page, "The browser says: SyntaxError");
});

test("a script that does not load is named", async ({ page }) => {
  await page.route("**/assets/index-*.js", (route) => route.abort());
  await page.goto("./");
  await expectFailed(page, /did not load: .*assets\/index-.*\.js/);
});

test("storage that fails once the app is drawing is said", async ({ page }) => {
  await page.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw new DOMException("denied", "SecurityError");
    };
  });
  await page.goto("./");
  await expectFailed(
    page,
    "The browser says: SecurityError: denied",
    "RetireTui stopped working",
  );
});

test("a page that throws is said inside the shell, which goes on working", async ({
  page,
}) => {
  await page.route("**/assets/overview-*.js", (route) => route.abort());
  await page.goto("./");
  await page.getByRole("button", { name: /^Starter/ }).click();
  await expect(
    page.getByRole("heading", { name: "This page stopped working" }),
  ).toBeVisible();
  await expect(
    page.getByRole("main").getByRole("link", { name: "Report an issue" }),
  ).toHaveAttribute("href", /\/retiretui\/issues\/new\?body=/);
  await expect(page.locator("header").getByText("starter.toml")).toBeVisible();
  await page.getByRole("link", { name: "Ledger" }).click();
  await expect(page.getByRole("heading", { name: "Ledger" })).toBeVisible();
});
