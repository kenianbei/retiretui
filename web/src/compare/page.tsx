import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  compareHeaders,
  compareWords,
  yearAmong,
  type CompareView,
  type Document,
  type Searched,
  type YearFigure,
} from "@wasm/retiretui_wasm.js";
import { GitCompareArrows } from "lucide-react";
import { useMemo } from "react";

import { withIn } from "@/compare/search";
import { Changes, PlanActions, Plans, type PlanEntry } from "@/compare/plans";
import {
  laneOf,
  useCompared,
  useReleased,
  type ComparedFile,
} from "@/compare/use-compared";
import { Views, type Charted } from "@/compare/views";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { messageOf } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/words";
import { useSuccesses } from "@/searches";
import { useSession } from "@/session";
import { nameOf } from "@/workspace";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";

const WORDS = compareWords();
/** The metric shown where the address names none. */
const NET_WORTH = "net-worth";
/** The lane the Overview searches the document in, shared with it. */
const DOCUMENT_LANE = "monteCarlo";

/** The plan's text to search, where it has no issues. */
function searchedText(document: Document | null): string {
  if (!document || document.issues().length > 0) return "";
  return document.planText();
}

/** What `run` answers, or why there is none. */
function attempt<T>(run: () => T): T | string {
  try {
    return run();
  } catch (thrown) {
    return messageOf(thrown);
  }
}

/** The files beside the document to compare it with, ticked while they are. */
function CompareWith({
  offered,
  compared,
  onCompared,
}: {
  offered: readonly string[];
  compared: readonly string[];
  onCompared: (paths: string[]) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" disabled={offered.length === 0}>
          <GitCompareArrows aria-hidden />
          Compare with
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {offered.map((path) => (
          <DropdownMenuCheckboxItem
            key={path}
            checked={compared.includes(path)}
            onSelect={(event) => {
              event.preventDefault();
            }}
            onCheckedChange={(isChecked) => {
              onCompared(
                isChecked
                  ? [...compared, path]
                  : compared.filter((each) => each !== path),
              );
            }}
          >
            {nameOf(path)}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Where a plan's search through random markets has got to. */
function searchedOf(
  found:
    | {
        data?: { success_rate: number };
        error: Error | null;
        isFetching: boolean;
      }
    | undefined,
): Searched {
  if (found?.data) return { kind: "rate", rate: found.data.success_rate };
  if (found?.error && !found.isFetching) return { kind: "failed" };
  return { kind: "waiting" };
}

/** A plan compared, and where its search through random markets has got to. */
type Row = ComparedFile & { searched: Searched };

/** A plan's first and last years, as numbers. */
function yearsOf(row: Row): number[] {
  return Array.from(row.document?.years() ?? []);
}

/**
 * The document beside the workspace files compared with it: each plan's
 * figures, or its differences from the baseline, what the highlighted one
 * changes of the baseline, and a metric year by year.
 */
export function ComparePage() {
  const session = useSession();
  const { reading, files } = session;
  const own = session.path ?? "";
  const search = useSearch({ from: "/compare" });
  const navigate = useNavigate({ from: "/compare" });
  const compared = useMemo(
    () => (search.with ?? []).filter((path) => path !== own),
    [search.with, own],
  );
  const opened = useCompared(compared);
  const lanes = useMemo(() => compared.map(laneOf), [compared]);
  useReleased(lanes);
  const found = useSuccesses(
    [
      { plan: searchedText(reading.document), lane: DOCUMENT_LANE },
      ...opened.map((plan) => ({
        plan: searchedText(plan.document),
        lane: laneOf(plan.path),
      })),
    ],
    true,
  );
  const document: Row = {
    path: own,
    document: reading.document,
    error: session.error,
    searched: searchedOf(found[0]),
  };
  const rows: Row[] = [
    document,
    ...opened.map((plan, at) => ({
      ...plan,
      searched: searchedOf(found[at + 1]),
    })),
  ];

  const at = (path: string | undefined) =>
    Math.max(
      0,
      rows.findIndex((row) => row.path === path),
    );
  const baselineAt = search.baseline === undefined ? 0 : at(search.baseline);
  const highlightedAt = search.plan === undefined ? 0 : at(search.plan);
  const isDifference = search.difference === true && rows.length > 1;
  const baseline = rows[baselineAt] ?? document;
  const basis = basisOf(search);
  const metric =
    WORDS.metrics.find((each) => each.key === search.metric)?.key ?? NET_WORTH;
  const spans = rows.flatMap(yearsOf);
  const year = yearAmong(
    search.year ?? null,
    new Date().getFullYear(),
    Int16Array.from(yearsOf(document)),
    Int16Array.from(
      spans.length === 0 ? [] : [Math.min(...spans), Math.max(...spans)],
    ),
  );
  const view: CompareView = { nominal: basis === "nominal", metric, year };
  const against = (place: number) =>
    isDifference && place !== baselineAt ? baseline.document : null;

  const entries: PlanEntry[] = rows.map((row, place) => ({
    path: row.path,
    name: nameOf(row.path),
    cells: attempt(() => {
      if (!row.document) return row.error ?? "";
      const base = against(place);
      return base
        ? row.document.planFiguresAgainst(
            view,
            row.searched,
            base,
            baseline.searched,
          )
        : row.document.planFigures(view, row.searched);
    }),
  }));
  const charted: Charted[] = rows.map((row, place) => ({
    name: nameOf(row.path),
    isAlongZero: isDifference && place === baselineAt,
    figures: attempt<YearFigure[]>(() => {
      if (!row.document) return [];
      const base = against(place);
      return base
        ? row.document.byYearAgainst(view, base)
        : row.document.byYear(view);
    }),
  }));

  const highlightedRow = rows[highlightedAt] ?? document;
  const highlighted = entries[highlightedAt] ?? {
    path: own,
    name: nameOf(own),
    cells: [],
  };
  const changes = changesOf(
    highlightedRow,
    baseline,
    highlightedAt === baselineAt,
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

  return (
    <div className="max-w-6xl space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Compare</h1>
        <div className="flex flex-wrap items-center gap-2">
          <CompareWith
            offered={files.filter((path) => path !== own)}
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
        headers={compareHeaders(metric, year)}
        entries={entries}
        highlighted={highlighted}
        highlight={(entry) => {
          place({ plan: pathOr(entry.path) });
        }}
        caption={`Plans${measured} · ${unit}`}
      />
      <PlanActions
        isDocument={highlightedAt === 0}
        isBaseline={highlightedAt === baselineAt}
        canOpen={highlightedRow.document !== null}
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
              ? `Changes · ${highlighted.name}`
              : `Changes · ${highlighted.name} against ${nameOf(baseline.path)}`
          }
          lines={changes.lines}
          isNote={changes.isNote}
        />
        <Views
          plans={charted}
          words={WORDS}
          metric={metric}
          view={search.view ?? "chart"}
          caption={`${WORDS.metrics.find((each) => each.key === metric)?.title ?? ""} by year${measured} · ${unit}`}
          year={year}
          onYear={(year) => {
            place({ year });
          }}
          onView={(view) => {
            place({ view: view === "table" ? view : undefined });
          }}
          onMetric={(metric) => {
            place({
              metric: metric === NET_WORTH ? undefined : metric,
            });
          }}
        />
      </div>
    </div>
  );
}

/** What `own` changes of `base`, or a note saying why that is not said. */
function changesOf(
  own: ComparedFile,
  base: ComparedFile,
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
