import { Link } from "@tanstack/react-router";
import type { OverviewView } from "@wasm/retiretui_wasm.js";
import type { ReactNode } from "react";

import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { MONTE_CARLO } from "@/nav";
import { BASIS_LABEL, VIEW_WORDS } from "@/overview/view-words";
import type { Basis } from "@/overview/words";
import { useMarkets } from "@/searches";
import { ZONE_CLASS } from "@/tools/markets/zone";
import { keptSearch } from "@/year/search";

const [MONEY_LASTS, SUCCESS, ENDS_WITH, LIFETIME_TAXES] = VIEW_WORDS.strip;

/**
 * A reading of the strip: its label beside its figure in a narrow strip,
 * over it in a wide one, and what the figure is in beneath it.
 */
function Reading({
  label,
  caption,
  children,
}: {
  label: string;
  caption?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="bg-card flex items-baseline justify-between gap-4 px-4 py-3 @md:block @md:space-y-1">
      <dt className="text-muted-foreground text-sm">{label}</dt>
      <dd className="text-right @md:text-left">
        <div className="text-lg font-semibold tabular-nums @md:text-2xl">
          {children}
        </div>
        {caption && (
          <div className="text-muted-foreground text-xs">{caption}</div>
        )}
      </dd>
    </div>
  );
}

/** How surely the money lasts through random markets, leading to the tool. */
function Success({ plan }: { plan: string | null }) {
  const markets = useMarkets("monteCarlo", plan ?? "", plan !== null);
  if (plan === null) {
    return (
      <Reading label={SUCCESS} caption="once the plan's issues are fixed">
        —
      </Reading>
    );
  }
  if (markets.error) {
    return (
      <Reading label={SUCCESS}>
        <span className="text-muted-foreground text-sm font-normal">
          {markets.error.message}
        </span>
      </Reading>
    );
  }
  if (!markets.data || markets.isPlaceholderData) {
    return (
      <Reading label={SUCCESS} caption="Running markets…">
        <Skeleton className="ml-auto h-7 w-28 @md:ml-0 @md:h-8" />
      </Reading>
    );
  }
  return (
    <Reading label={SUCCESS} caption="through random markets">
      <Link
        to="/tools/$page"
        params={{ page: MONTE_CARLO }}
        search={(kept) => keptSearch(kept, ["basis", "held"])}
        className={cn(
          "underline-offset-4 hover:underline",
          ZONE_CLASS[markets.data.zone],
        )}
      >
        {markets.data.success}
      </Link>
    </Reading>
  );
}

/** The verdict: how long the money lasts, how surely, what it ends with and pays in tax. */
export function Strip({
  view,
  basis,
  plan,
}: {
  view: OverviewView;
  basis: Basis;
  plan: string | null;
}) {
  const unit = BASIS_LABEL[basis];
  return (
    <div className="@container">
      <dl className="bg-border grid gap-px overflow-hidden rounded-xl border @md:grid-cols-2 @6xl:grid-cols-4">
        <Reading label={MONEY_LASTS}>
          <span className={cn(view.shortfall && "text-destructive")}>
            {view.money_lasts}
          </span>
        </Reading>
        <Success plan={plan} />
        <Reading label={ENDS_WITH} caption={unit}>
          {view.ends_with}
        </Reading>
        <Reading label={LIFETIME_TAXES} caption={unit}>
          {view.lifetime_taxes}
        </Reading>
      </dl>
    </div>
  );
}
