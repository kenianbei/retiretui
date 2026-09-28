import { useParams } from "@tanstack/react-router";

import { ROTH_CONVERSIONS, SSA_BENEFITS } from "@/nav";
import { GroupedPage } from "@/pages/placeholder";
import { ClaimsPage } from "@/tools/claims/page";
import { ConversionsPage } from "@/tools/conversions/page";

/** The tool its route's `$page` named; one still to be built says so. */
export function ToolPage() {
  const { page } = useParams({ from: "/tools/$page" });
  if (page === ROTH_CONVERSIONS) return <ConversionsPage />;
  if (page === SSA_BENEFITS) return <ClaimsPage />;
  return <GroupedPage />;
}
