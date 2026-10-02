import { version } from "@wasm/retiretui_wasm.js";

/** The project, where its source is read. */
export const REPO_URL = "https://github.com/kenianbei/retiretui";

/** Where a report of something wrong begins. */
export const ISSUES_URL = `${REPO_URL}/issues/new`;

/**
 * Where a report begins once the app runs: what is running and in which
 * browser written in for the reporter, and nothing of a plan.
 */
export function reportUrl(): string {
  const body = [
    "<!-- What happened, and what did you expect? Nothing of your plan is sent with this. -->",
    "",
    "",
    "---",
    `RetireTui ${version()}, the web app`,
    navigator.userAgent,
  ].join("\n");
  return `${ISSUES_URL}?body=${encodeURIComponent(body)}`;
}
