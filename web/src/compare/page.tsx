import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  compareHeaders,
  compareWords,
  yearAmong,
  type CompareView,
  type YearFigure,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import {
  Changes,
  CompareWith,
  PlanActions,
  Plans,
  type PlanEntry,
} from "@/compare/plans";
import { withIn } from "@/compare/search";
import { useRows, type Row } from "@/compare/use-compared";
import { Views, type Charted } from "@/compare/views";
import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { nameOf } from "@/workspace";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";

const WORDS = compareWords();
/** The metric shown where the address names none. */
const NET_WORTH = "net-worth";

/** What `run` answers, or why there is none. */
function attempt<T>(run: () => T): T | string {
  try {
    return run();
  } catch (thrown) {
    return messageOf(thrown);
  }
}

/** The year shown, held within the document's years, then within any plan's. */
function yearOf(rows: readonly Row[], requested: number | undefined): number {
  const spans = rows.flatMap((row) => Array.from(row.document?.years() ?? []));
  const shows =
    spans.length === 0 ? [] : [Math.min(...spans), Math.max(...spans)];
  return yearAmong(
    requested ?? null,
    new Date().getFullYear(),
    rows[0]?.document?.years() ?? new Int16Array(),
    Int16Array.from(shows),
  );
}

/** What `own` changes of `base`, or a note saying why that is not said. */
function changesOf(
  own: Row,
  base: Row,
  isBaseline: boolean,
): { lines: string[]; isNote: boolean } {
  if (isBaseline) return { lines: [WORDS.the_baseline], isNote: true };
  if (!own.document || !base.document) {
    return { lines: [own.error ?? base.error ?? ""], isNote: true };
  }
  const { document } = own;
  const baseDocument = base.document;
  const lines = attempt(() => document.changesFrom(baseDocument));
  if (typeof lines === "string") return { lines: [lines], isNote: true };
  if (lines.length === 0) return { lines: [WORDS.the_same], isNote: true };
  return { lines, isNote: false };
}

/** Each plan's row of figures, and its metric year by year, in `view`. */
function figuresOf(
  rows: readonly Row[],
  view: CompareView,
  baseline: { at: number; isDifference: boolean },
): { entries: PlanEntry[]; charted: Charted[] } {
  const base = rows[baseline.at] ?? rows[0];
  const against = (place: number) =>
    baseline.isDifference && place !== baseline.at ? base?.document : null;
  const entries = rows.map((row, place) => ({
    path: row.path,
    name: nameOf(row.path),
    cells: attempt(() => {
      if (!row.document) return row.error ?? "";
      const other = against(place);
      return other && base
        ? row.document.planFiguresAgainst(
            view,
            row.searched,
            other,
            base.searched,
          )
        : row.document.planFigures(view, row.searched);
    }),
  }));
  const charted = rows.map((row, place) => ({
    name: nameOf(row.path),
    isAlongZero: baseline.isDifference && place === baseline.at,
    figures: attempt<YearFigure[]>(() => {
      if (!row.document) return [];
      const other = against(place);
      return other
        ? row.document.byYearAgainst(view, other)
        : row.document.byYear(view);
    }),
  }));
  return { entries, charted };
}

/**
 * The document beside the workspace files compared with it: each plan's
 * figures, or its differences from the baseline, what the highlighted one
 * changes of the baseline, and a metric year by year.
 */
export function ComparePage() {
  const session = useSession();
  const own = session.path ?? "";
  const search = useSearch({ from: "/compare" });
  const navigate = useNavigate({ from: "/compare" });
  const compared = useMemo(
    () => (search.with ?? []).filter((path) => path !== own),
    [search.with, own],
  );
  const rows = useRows(compared);
  const [document] = rows;
  const at = (path: string | undefined) =>
    Math.max(
      0,
      rows.findIndex((row) => row.path === path),
    );
  const baselineAt = at(search.baseline);
  const highlightedAt = at(search.plan);
  const baseline = rows[baselineAt] ?? document;
  const highlighted = rows[highlightedAt] ?? document;
  const isDifference = search.difference === true && rows.length > 1;
  const basis = basisOf(search);
  const metric =
    WORDS.metrics.find((each) => each.key === search.metric) ??
    WORDS.metrics[0];
  const metricKey = metric?.key ?? NET_WORTH;
  const year = yearOf(rows, search.year);
  const { entries, charted } = useMemo(
    () =>
      figuresOf(
        rows,
        { nominal: basis === "nominal", metric: metricKey, year },
        { at: baselineAt, isDifference },
      ),
    [rows, basis, metricKey, year, baselineAt, isDifference],
  );
  const changes = useMemo(
    () => changesOf(highlighted, baseline, highlightedAt === baselineAt),
    [highlighted, baseline, highlightedAt, baselineAt],
  );
  const unit = BASIS_LABEL[basis];
  const measured = isDifference ? ` · against ${nameOf(baseline.path)}` : "";
  const place = (search: Record<string, unknown>) => {
    void navigate({
      search: (prev) => ({ ...prev, ...search }),
      replace: true,
    });
  };
  const pathOr = (path: string) => (path === own ? undefined : path);
  const name = nameOf(highlighted.path);

  return (
    <div className="max-w-6xl space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Compare</h1>
        <div className="flex flex-wrap items-center gap-2">
          <CompareWith
            offered={session.files.filter((path) => path !== own)}
            compared={compared}
            onCompared={(paths) => {
              place({ with: withIn(paths) });
            }}
          />
          <Button
            variant="outline"
            aria-pressed={isDifference}
            disabled={rows.length < 2}
            className={isDifference ? "bg-accent" : undefined}
            onClick={() => {
              place({ difference: isDifference ? undefined : true });
            }}
          >
            Difference
          </Button>
          <BasisSwitch />
        </div>
      </div>
      {compared.length === 0 && (
        <p className="text-muted-foreground">
          Compare this plan with others in the workspace: Compare with adds one.
        </p>
      )}
      <Plans
        headers={compareHeaders(metricKey, year)}
        entries={entries}
        highlighted={entries[highlightedAt]}
        highlight={(entry) => {
          place({ plan: pathOr(entry.path) });
        }}
        caption={`Plans${measured} · ${unit}`}
      />
      <PlanActions
        isDocument={highlightedAt === 0}
        isBaseline={highlightedAt === baselineAt}
        canOpen={highlighted.document !== null}
        onOpen={() => {
          session.open(highlighted.path);
        }}
        onBaseline={() => {
          place({ baseline: pathOr(highlighted.path) });
        }}
        onRemove={() => {
          place({
            with: withIn(compared.filter((path) => path !== highlighted.path)),
            plan: undefined,
          });
        }}
      />
      <div className="grid items-start gap-4 lg:grid-cols-[minmax(0,20rem)_minmax(0,1fr)]">
        <Changes
          title={
            highlightedAt === baselineAt
              ? `Changes · ${name}`
              : `Changes · ${name} against ${nameOf(baseline.path)}`
          }
          lines={changes.lines}
          isNote={changes.isNote}
        />
        <Views
          plans={charted}
          words={WORDS}
          metric={metricKey}
          view={search.view ?? "chart"}
          caption={`${metric?.title ?? ""} by year${measured} · ${unit}`}
          year={year}
          onYear={(year) => {
            place({ year });
          }}
          onView={(view) => {
            place({ view: view === "table" ? view : undefined });
          }}
          onMetric={(metric) => {
            place({ metric: metric === NET_WORTH ? undefined : metric });
          }}
        />
      </div>
    </div>
  );
}
