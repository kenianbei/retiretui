import { version } from "@wasm/retiretui_wasm.js";

import { REPO_URL, reportUrl } from "@/links";

const LINK =
  "hover:text-foreground inline-block py-2 underline underline-offset-4";

/** Under every page: that this is a model, and the ways out of the app. */
export function Footer() {
  return (
    <footer className="text-muted-foreground flex flex-wrap items-center justify-between gap-x-8 border-t px-4 py-3 text-sm md:px-8">
      <p className="py-2">
        RetireTui is a model under the assumptions you give it, not financial
        advice.
      </p>
      <ul className="flex flex-wrap items-center gap-x-5">
        <li>
          <a
            href={reportUrl()}
            target="_blank"
            rel="noreferrer"
            className={LINK}
          >
            Report an issue
          </a>
        </li>
        <li>
          <a href={REPO_URL} target="_blank" rel="noreferrer" className={LINK}>
            Source on GitHub
          </a>
        </li>
        <li>Version {version()}</li>
      </ul>
    </footer>
  );
}
