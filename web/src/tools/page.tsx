import { useParams } from "@tanstack/react-router";

import { ROTH_CONVERSIONS } from "@/nav";
import { GroupedPage } from "@/pages/placeholder";
import { ConversionsPage } from "@/tools/conversions/page";

/** The tool its route's `$page` named; one still to be built says so. */
export function ToolPage() {
  const { page } = useParams({ from: "/tools/$page" });
  return page === ROTH_CONVERSIONS ? <ConversionsPage /> : <GroupedPage />;
}
