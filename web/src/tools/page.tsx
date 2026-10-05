import { useParams } from "@tanstack/react-router";

import {
  HISTORICAL,
  MONTE_CARLO,
  pageOf,
  ROTH_CONVERSIONS,
  SPENDING_CEILING,
  SSA_BENEFITS,
  TAX_TABLES,
  TOOLS,
  WITHDRAWAL_ORDER,
} from "@/nav";
import { ClaimsPage } from "@/tools/claims/page";
import { ConversionsPage } from "@/tools/conversions/page";
import { MarketsPage } from "@/tools/markets/page";
import { OrdersPage } from "@/tools/orders/page";
import { SpendingPage } from "@/tools/spending/page";
import { TaxTablesPage } from "@/tools/tax/page";

/** The tool its route's `$page` named. */
export function ToolPage() {
  const { page } = useParams({ from: "/tools/$page" });
  if (page === ROTH_CONVERSIONS) return <ConversionsPage />;
  if (page === SSA_BENEFITS) return <ClaimsPage />;
  const { title } = pageOf(TOOLS, page);
  if (page === WITHDRAWAL_ORDER) return <OrdersPage title={title} />;
  if (page === MONTE_CARLO) {
    return <MarketsPage key={page} kind="monteCarlo" title={title} />;
  }
  if (page === HISTORICAL) {
    return <MarketsPage key={page} kind="historical" title={title} />;
  }
  if (page === SPENDING_CEILING) return <SpendingPage title={title} />;
  if (page === TAX_TABLES) return <TaxTablesPage title={title} />;
  return null;
}
