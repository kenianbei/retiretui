import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  spendingWords,
  type CeilingOption,
  type SpendingOptions,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { MarginNote } from "@/components/margin-note";
import type { Basis } from "@/overview/words";
import { useSpending } from "@/searches";
import { placeSearch } from "@/plan/search";
import { useSession } from "@/session";
import { ToolAbout } from "@/tools/about";
import { SearchActions, type Chosen } from "@/tools/act";
import { Options, type OptionRow } from "@/tools/options";
import type { ToolSearch } from "@/tools/search";
import { SettingsForm, SettingsRead, type Settings } from "@/tools/settings";
import { offeredName } from "@/workspace";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";

const WORDS = spendingWords();

/** The ceiling a person is after, highlighted where the address names none. */
const AT_TARGET = "target";
/** The ceiling the note is about. */
const PLANNED = "planned";
/** The column a phone's row shows beside what a ceiling was held to. */
const FLEXIBLE = 1;
/** Where what a plan must end with is edited. */
const LEAVE_AT_LEAST = {
  domain: "market",
  index: null,
  field: "leave_at_least",
};

/** The share of random markets the ceiling is held to. */
const TARGET: Settings = {
  heading: "Target",
  read: (document) => document.targetRead(),
  open: (document) => document.target(),
  apply: (document, editor) => {
    document.applyTarget(editor);
  },
};

const column = columnsFor<[string, string, string]>();
const ITEM_COLUMNS = WORDS.item_columns.map((header, at) =>
  column.display({
    id: String(at),
    header,
    meta: { isNumeric: at >= 1 },
    cell: ({ row }) => row.original[at],
  }),
);

/** The plan as it stands, then each ceiling. */
function rowsOf(
  found: SpendingOptions,
  basis: Basis,
): OptionRow<CeilingOption>[] {
  return [
    {
      key: found.current,
      cells: [found.current, ...found.baseline[basis]],
      narrow: found.current,
      option: null,
    },
    ...found.options.map((option) => ({
      key: option.key,
      cells: [option.held_to, ...option.figures[basis]],
      narrow: option.held_to,
      option,
    })),
  ];
}

/** The highlighted ceiling, as it is taken or written. */
function chosenCeiling(path: string | null, option: CeilingOption): Chosen {
  return {
    noun: "this ceiling",
    question: option.question,
    described:
      "This ceiling is written as a scenario over this plan, to a file of this name in your workspace.",
    offered: offeredName(path, `spending-${option.key}`),
    take: (document) => document.takeSpending(option.expenses),
    scenario: (document, out) =>
      document.spendingScenario(out, option.expenses),
  };
}

/** What the search is handed: the draft as text, and the target it holds. */
function useSearched() {
  const { reading, issues } = useSession();
  const isValid = issues.length === 0;
  return useMemo(() => {
    const document = reading.document;
    return {
      plan: document?.planText() ?? "",
      success: document?.targetShare ?? 0,
      isValid,
    };
  }, [reading, isValid]);
}

/**
 * The Spending Ceiling tool: the most the plan's flexible spending could be
 * in its own market and at the target, the highlighted ceiling's expenses,
 * and the ceiling taken or written as a scenario.
 */
export function SpendingPage({ title }: { title: string }) {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const basis = basisOf(search);
  const { path } = useSession();
  const { plan, success, isValid } = useSearched();
  const found = useSpending(plan, success, isValid);
  const reply = found.data;
  const wanted = search.ceiling ?? AT_TARGET;
  const highlighted = reply?.options.find((each) => each.key === wanted);
  const isCurrent = !found.isFetching && !found.isPlaceholderData;
  const rows = useMemo(() => reply && rowsOf(reply, basis), [reply, basis]);
  const isShown = reply && highlighted && isValid && !found.error;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          <ToolAbout about={WORDS.about} />
        </div>
        <BasisSwitch />
      </div>
      <section aria-labelledby="ceilings" className="min-w-0 space-y-3">
        <div className="flex items-baseline gap-3">
          <h2 id="ceilings" className="text-lg font-semibold">
            Ceilings
          </h2>
          {found.isFetching && (
            <span className="text-muted-foreground text-sm">Searching…</span>
          )}
        </div>
        {!isValid ? (
          <p className="text-muted-foreground">
            The ceilings are searched once the plan&apos;s issues are fixed; the{" "}
            <Link to="/overview" className="underline underline-offset-4">
              Overview
            </Link>{" "}
            lists them.
          </p>
        ) : found.error && !found.isFetching ? (
          <MarginNote zone="caution" role="alert">
            <p className="text-sm">{found.error.message}</p>
          </MarginNote>
        ) : !reply ? (
          <p className="text-muted-foreground">{WORDS.nothing_searched}</p>
        ) : (
          <Options
            label="Ceilings"
            columns={reply.columns}
            rows={rows ?? []}
            narrowFigure={FLEXIBLE}
            highlighted={highlighted}
            highlight={(option) => {
              void navigate({
                search: (kept) => ({ ...kept, ceiling: option.key }),
                replace: true,
              });
            }}
          />
        )}
      </section>
      <div className="grid grid-cols-1 items-start gap-6 @wide/page:grid-cols-2">
        {isShown && (
          <section aria-labelledby="expenses" className="min-w-0 space-y-3">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <h2 id="expenses" className="text-lg font-semibold">
                Expenses ({highlighted.held_to.toLowerCase()})
              </h2>
              <SearchActions
                key={highlighted.key}
                chosen={chosenCeiling(path, highlighted)}
                isCurrent={isCurrent}
              />
            </div>
            <DataTable
              label={`Expenses ${highlighted.held_to.toLowerCase()}`}
              columns={ITEM_COLUMNS}
              rows={highlighted.items}
              rowKey={(cells) => cells.join("|")}
            />
            {reply.note && highlighted.key === PLANNED && (
              <p className="text-muted-foreground text-sm">
                {reply.note}{" "}
                <Link
                  to="/plan/$page"
                  params={{ page: LEAVE_AT_LEAST.domain }}
                  search={placeSearch(LEAVE_AT_LEAST).search}
                  className="underline underline-offset-4"
                >
                  Edit Leave at least
                </Link>
              </p>
            )}
          </section>
        )}
        <SettingsRead settings={TARGET} />
      </div>
      {search.edit && <SettingsForm settings={TARGET} />}
    </div>
  );
}
