import { useSearch } from "@tanstack/react-router";
import { useMemo } from "react";

import { QUARTERS } from "@/lib/utils";

import { Better } from "@/overview/better";
import { EveryYear, PlanChart } from "@/overview/charts";
import { RowList } from "@/overview/lists";
import { Problems, Shortfall } from "@/overview/notes";
import { Strip } from "@/overview/strip";
import { ThisYear } from "@/overview/this-year";
import { VIEW_WORDS } from "@/overview/view-words";
import { useSession } from "@/session";
import { basisOf, heldOf } from "@/year/search";
import { useYear } from "@/year/use-year";
import { BasisSwitch } from "@/year/year";

const CHART_ORDER = ["balances", "net-worth", "income", "markets"] as const;

/**
 * The ledger's first page: whether the money lasts and how surely, what to
 * do in the year shown, what needs attention and when the big things
 * happen, what could do better, and the plan charted.
 */
export function Overview() {
  const { reading, document, issues } = useSession();
  const search = useSearch({ from: "/overview" });
  const basis = basisOf(search);
  const held = useMemo(() => heldOf({ held: search.held }), [search.held]);
  const isValid = issues.length === 0;
  const shown = useYear();
  const plan = useMemo(
    () => (isValid ? (reading.document?.planText() ?? null) : null),
    [reading, isValid],
  );
  const view = useMemo(
    () => reading.document?.overview(basis === "nominal") ?? null,
    [reading, basis],
  );
  const series = useMemo(
    () => reading.document?.chart(basis === "nominal"),
    [reading, basis],
  );
  if (!document) return null;
  const charts = series && {
    series,
    basis,
    plan,
    year: shown.year,
    onYear: shown.setYear,
  };

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
      <div className={QUARTERS}>
        <ThisYear shown={shown} basis={basis} />
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
            rows={view.attention}
            empty={VIEW_WORDS.nothing_wanting}
          />
        )}
        {plan !== null && <Better plan={plan} held={held} basis={basis} />}
      </div>
      {charts && (
        <div className="space-y-3">
          <div className={QUARTERS}>
            {CHART_ORDER.map((chart) => (
              <PlanChart key={chart} chart={chart} {...charts} />
            ))}
          </div>
          <EveryYear />
        </div>
      )}
    </div>
  );
}
