import { Link } from "@tanstack/react-router";
import { issueCount, type PlacedIssue } from "@wasm/retiretui_wasm.js";

import { MarginNote } from "@/components/margin-note";
import { IssueLink } from "@/draft/issue-link";
import { keptSearch } from "@/year/search";

const EXPENSES = "expenses";

/**
 * The draft's issues, each linked to its field, and what the figures below
 * are while it has them - or that there are none until they are fixed.
 */
export function Problems({
  issues,
  hasFigures,
}: {
  issues: readonly PlacedIssue[];
  hasFigures: boolean;
}) {
  return (
    <MarginNote zone="shortfall" role="region" aria-labelledby="problems">
      <h2 id="problems" className="font-semibold">
        This plan has {issueCount(issues.length)}
      </h2>
      <ul className="space-y-1 text-sm">
        {issues.map((issue) => (
          <li key={issue.words}>
            <IssueLink issue={issue} className="underline underline-offset-4" />
          </li>
        ))}
      </ul>
      <p className="text-muted-foreground text-sm">
        {hasFigures
          ? "The figures below are the last the plan had without them."
          : "Its figures show once they are fixed."}
      </p>
    </MarginNote>
  );
}

/** The year the money first runs short, leading to it in the Ledger and to what the plan spends. */
export function Shortfall({ year, said }: { year: number; said: string }) {
  const link = "text-sm underline underline-offset-4";
  return (
    <MarginNote zone="shortfall">
      <p className="font-semibold">{said}</p>
      <p className="flex flex-wrap gap-x-4 gap-y-1">
        <Link
          to="/ledger"
          search={(kept) => ({ ...keptSearch(kept, ["basis", "held"]), year })}
          className={link}
        >
          {year} in the Ledger
        </Link>
        <Link to="/plan/$page" params={{ page: EXPENSES }} className={link}>
          Expenses
        </Link>
      </p>
    </MarginNote>
  );
}
