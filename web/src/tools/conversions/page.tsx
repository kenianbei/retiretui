import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { ladderWords, type LadderOption } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { useLadders } from "@/searches";
import { useSession } from "@/session";
import { LadderActions } from "@/tools/conversions/act";
import { Constraints, ConstraintsForm } from "@/tools/conversions/constraints";
import { Conversions } from "@/tools/conversions/ladder";
import { Options } from "@/tools/conversions/options";
import { percentOf, type ToolSearch } from "@/tools/search";
import { basisOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";

const WORDS = ladderWords();

/** What the search is handed: the draft and its constraints, as text. */
function useSearched() {
  const { reading, issues } = useSession();
  const isValid = issues.length === 0;
  return useMemo(() => {
    const document = reading.document;
    return {
      plan: document?.planText() ?? "",
      constraints: document?.constraintsText ?? "",
      isAimed: document?.isAimed ?? false,
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
  const { plan, constraints, isAimed, isValid } = useSearched();
  const found = useLadders(plan, constraints, isAimed && isValid);
  const reply = found.data;
  const highlighted =
    reply?.brackets.find((each) => percentOf(each.rate) === search.bracket) ??
    reply?.brackets[0];
  const isCurrent = !found.isFetching && !found.isPlaceholderData;

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
    <div className="max-w-5xl space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">
          Roth Conversions
        </h1>
        <BasisSwitch />
      </div>
      <Constraints />
      <section aria-labelledby="options" className="space-y-3">
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
            The ladders are searched once the plan&apos;s issues are fixed; the{" "}
            <Link to="/overview" className="underline underline-offset-4">
              Overview
            </Link>{" "}
            lists them.
          </p>
        ) : found.error && !found.isFetching ? (
          <Alert variant="destructive">
            <AlertDescription>{found.error.message}</AlertDescription>
          </Alert>
        ) : reply && reply.brackets.length === 0 ? (
          <p className="text-muted-foreground">{WORDS.no_bracket}</p>
        ) : (
          reply && (
            <Options
              found={reply}
              basis={basis}
              highlighted={highlighted}
              highlight={highlight}
            />
          )
        )}
      </section>
      {reply && highlighted && isAimed && isValid && !found.error && (
        <section aria-labelledby="conversions" className="space-y-3">
          <h2 id="conversions" className="text-lg font-semibold">
            Conversions ({highlighted.label})
          </h2>
          <Conversions
            option={highlighted}
            headers={reply.conversion_columns}
            basis={basis}
            nothing={WORDS.converts_nothing}
          />
          <LadderActions
            key={highlighted.label}
            destination={reply.destination}
            option={highlighted}
            isCurrent={isCurrent}
          />
        </section>
      )}
      {search.edit && <ConstraintsForm />}
    </div>
  );
}
