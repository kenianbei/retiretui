import { useNavigate, useSearch } from "@tanstack/react-router";
import { useMemo } from "react";
import { HISTORICAL } from "@/nav";
import { Better } from "@/overview/better";
import { PlanChart } from "@/overview/charts";
import { RowList, ToolRow } from "@/overview/lists";
import { Problems, Shortfall } from "@/overview/notes";
import { chartOf } from "@/overview/search";
import { Strip } from "@/overview/strip";
import { OverThePlan, RestsOn } from "@/overview/totals";
import { VIEW_WORDS } from "@/overview/view-words";
import { useMarkets } from "@/searches";
import { useSession } from "@/session";
import { basisOf, heldOf, keptSearch } from "@/year/search";
import { BasisSwitch } from "@/year/year";

/** Lists three across on a wide page, one under another on a narrower one. */
const THIRDS = "grid grid-cols-1 items-start gap-6 @4xl/page:grid-cols-3";

/**
 * The plan as a whole, nothing on it chosen by a year: whether the money
 * lasts and how surely, when the big things happen, what needs attention
 * and what could do better, then the plan charted beside what its years
 * add up to and what it rests on.
 */
export function Overview() {
  const { reading, document, issues } = useSession();
  const search = useSearch({ from: "/overview" });
  const navigate = useNavigate({ from: "/overview" });
  const basis = basisOf(search);
  const held = useMemo(() => heldOf({ held: search.held }), [search.held]);
  const isValid = issues.length === 0;
  const plan = useMemo(
    () => (isValid ? (reading.document?.planText() ?? null) : null),
    [reading, isValid],
  );
  const starts = useMarkets("historical", plan ?? "", plan !== null);
  const failing = starts.isPlaceholderData ? null : starts.data?.failing;
  const view = useMemo(
    () => reading.document?.overview(basis === "nominal") ?? null,
    [reading, basis],
  );
  const series = useMemo(
    () => reading.document?.chart(chartOf(search.chart), basis === "nominal"),
    [reading, basis, search.chart],
  );
  if (!document) return null;

  return (
    <div className="space-y-6">
      <div className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
          {view && <BasisSwitch />}
        </div>
        {!isValid && <Problems issues={issues} hasFigures={view !== null} />}
        {view?.shortfall && <Shortfall shortfall={view.shortfall} />}
        {view && <Strip view={view} basis={basis} plan={plan} />}
      </div>
      <div className={THIRDS}>
        {view && view.milestones.length > 0 && (
          <RowList
            id="milestones"
            title={VIEW_WORDS.milestones}
            rows={view.milestones}
          />
        )}
        {view && (
          <RowList
            id="attention"
            title={VIEW_WORDS.attention}
            lead={
              failing && (
                <ToolRow page={HISTORICAL} run={failing.key}>
                  {failing.said}
                </ToolRow>
              )
            }
            rows={view.attention}
            empty={VIEW_WORDS.nothing_wanting}
          />
        )}
        {plan !== null && <Better plan={plan} held={held} basis={basis} />}
      </div>
      {view && series && (
        <div className={THIRDS}>
          <div className="min-w-0 @4xl/page:col-span-2">
            <PlanChart
              chart={chartOf(search.chart)}
              series={series}
              basis={basis}
              plan={plan}
              onYear={(year) => {
                void navigate({
                  to: "/ledger",
                  search: (kept) => ({
                    ...keptSearch(kept, ["basis", "held"]),
                    year,
                  }),
                });
              }}
            />
          </div>
          <div className="min-w-0 space-y-6">
            <OverThePlan totals={view.totals} basis={basis} />
            <RestsOn rows={view.rests_on} />
          </div>
        </div>
      )}
    </div>
  );
}
