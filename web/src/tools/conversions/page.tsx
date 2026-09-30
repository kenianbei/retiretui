import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  ladderWords,
  type LadderOption,
  type LaddersReply,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { MarginNote } from "@/components/margin-note";
import { useLadders } from "@/searches";
import { useSession } from "@/session";
import { SearchActions, type Chosen } from "@/tools/act";
import { Constraints, ConstraintsForm } from "@/tools/conversions/constraints";
import { Conversions } from "@/tools/conversions/ladder";
import { Options, type OptionRow } from "@/tools/options";
import { percentOf, type ToolSearch } from "@/tools/search";
import type { Basis } from "@/overview/words";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";
import { offeredName } from "@/workspace";
import { ToolAbout } from "@/tools/about";

const WORDS = ladderWords();

/** The plan as it stands, then every bracket's ladder. */
function rowsOf(found: LaddersReply, basis: Basis): OptionRow<LadderOption>[] {
  const row = (
    label: string,
    figures: string[],
    option: LadderOption | null,
  ) => ({
    key: label,
    cells: [label, ...figures],
    narrow: label,
    option,
  });
  return [
    row(found.current, found.baseline[basis], null),
    ...found.brackets.map((option) =>
      row(option.label, option.figures[basis], option),
    ),
  ];
}

/** The highlighted ladder into `destination`, as it is taken or written. */
function chosenLadder(
  path: string | null,
  destination: string,
  option: LadderOption,
): Chosen {
  return {
    noun: "this ladder",
    question: option.question,
    described: `The ${option.label} ladder is written as a scenario over this plan, to a file of this name in your workspace.`,
    offered: offeredName(path, `ladder-${String(percentOf(option.rate))}`),
    take: (document) => document.takeLadder(destination, option.steps),
    scenario: (document, out) =>
      document.ladderScenario(out, destination, option.steps),
  };
}

/** What the search is handed: the draft and its constraints, as text. */
function useSearched() {
  const { reading, issues } = useSession();
  const isValid = issues.length === 0;
  return useMemo(() => {
    const document = reading.document;
    return {
      plan: document?.planText() ?? "",
      constraints: document?.constraintsText ?? "",
      destination: document?.destination,
      isValid,
    };
  }, [reading, isValid]);
}

/**
 * The Roth Conversions tool: the constraints, every bracket's ladder
 * searched under them best first, and the highlighted one year by year.
 */
export function ConversionsPage() {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const basis = basisOf(search);
  const { path } = useSession();
  const { plan, constraints, destination, isValid } = useSearched();
  const isAimed = destination !== undefined;
  const found = useLadders(
    plan,
    constraints,
    destination ?? "",
    isAimed && isValid,
  );
  const reply = found.data;
  const highlighted =
    reply?.brackets.find((each) => percentOf(each.rate) === search.bracket) ??
    reply?.brackets[0];
  const isCurrent = !found.isFetching && !found.isPlaceholderData;
  const rows = useMemo(() => reply && rowsOf(reply, basis), [reply, basis]);

  const highlight = (option: LadderOption) => {
    void navigate({
      search: (held) => ({
        ...held,
        bracket: percentOf(option.rate),
      }),
      replace: true,
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">
            Roth Conversions
          </h1>
          <ToolAbout about={WORDS.about} />
        </div>
        <BasisSwitch />
      </div>
      <div className="grid items-start gap-6 @wide/page:grid-cols-[auto_minmax(0,1fr)]">
        <section aria-labelledby="options" className="min-w-0 space-y-3">
          <div className="flex items-baseline gap-3">
            <h2 id="options" className="text-lg font-semibold">
              Ladder options
            </h2>
            {found.isFetching && (
              <span className="text-muted-foreground text-sm">Searching…</span>
            )}
          </div>
          {!isAimed ? (
            <p className="text-muted-foreground">{WORDS.pick_destination}</p>
          ) : !isValid ? (
            <p className="text-muted-foreground">
              The ladders are searched once the plan&apos;s issues are fixed;
              the{" "}
              <Link to="/overview" className="underline underline-offset-4">
                Overview
              </Link>{" "}
              lists them.
            </p>
          ) : found.error && !found.isFetching ? (
            <MarginNote zone="caution" role="alert">
              <p className="text-sm">{found.error.message}</p>
            </MarginNote>
          ) : reply && reply.brackets.length === 0 ? (
            <p className="text-muted-foreground">{WORDS.no_bracket}</p>
          ) : (
            reply && (
              <Options
                label="Ladder options"
                columns={reply.columns}
                rows={rows ?? []}
                narrowFigure={reply.columns.indexOf(WORDS.against_plan)}
                highlighted={highlighted}
                highlight={highlight}
              />
            )
          )}
        </section>
        {reply && highlighted && isAimed && isValid && !found.error && (
          <section
            aria-labelledby="conversions"
            className="w-fit max-w-full min-w-0 space-y-3"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <h2 id="conversions" className="text-lg font-semibold">
                Conversions ({highlighted.label})
              </h2>
              <SearchActions
                key={highlighted.label}
                chosen={chosenLadder(path, reply.destination, highlighted)}
                isCurrent={isCurrent}
              />
            </div>
            <Conversions
              option={highlighted}
              headers={reply.conversion_columns}
              basis={basis}
              nothing={WORDS.converts_nothing}
            />
          </section>
        )}
      </div>
      <Constraints />
      {search.edit && <ConstraintsForm />}
    </div>
  );
}
