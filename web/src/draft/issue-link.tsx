import { Link } from "@tanstack/react-router";
import type { PlacedIssue } from "@wasm/retiretui_wasm.js";

import { placeSearch } from "@/plan/search";

/**
 * An issue in the forms' words, linked - styled by `className` - to its field
 * where a form edits it.
 */
export function IssueLink({
  issue,
  className,
}: {
  issue: PlacedIssue;
  className?: string;
}) {
  if (!issue.place) return <span>{issue.words}</span>;
  const { page, search } = placeSearch(issue.place);
  return (
    <Link
      to="/plan/$page"
      params={{ page }}
      search={search}
      className={className}
    >
      {issue.words}
    </Link>
  );
}
