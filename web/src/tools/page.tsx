import { useParams } from "@tanstack/react-router";

import {
  HISTORICAL,
  MONTE_CARLO,
  ROTH_CONVERSIONS,
  SSA_BENEFITS,
  TOOLS,
  pageOf,
} from "@/nav";
import { GroupedPage } from "@/pages/placeholder";
import { ClaimsPage } from "@/tools/claims/page";
import { ConversionsPage } from "@/tools/conversions/page";
import { MarketsPage } from "@/tools/markets/page";

/** The tool its route's `$page` named; one still to be built says so. */
export function ToolPage() {
  const { page } = useParams({ from: "/tools/$page" });
  if (page === ROTH_CONVERSIONS) return <ConversionsPage />;
  if (page === SSA_BENEFITS) return <ClaimsPage />;
  const { title } = pageOf(TOOLS, page);
  if (page === MONTE_CARLO) {
    return <MarketsPage key={page} kind="monteCarlo" title={title} />;
  }
  if (page === HISTORICAL) {
    return <MarketsPage key={page} kind="historical" title={title} />;
  }
  return <GroupedPage />;
}
