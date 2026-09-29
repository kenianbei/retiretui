import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { keptSearch } from "@/year/search";

/** A link to `year` in the Ledger, the plan's own market, the basis and held claims kept. */
export function YearInLedger({
  year,
  className,
  children,
}: {
  year: number;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Link
      to="/ledger"
      search={(kept) => ({ ...keptSearch(kept, ["basis", "held"]), year })}
      className={className}
    >
      {children}
    </Link>
  );
}
