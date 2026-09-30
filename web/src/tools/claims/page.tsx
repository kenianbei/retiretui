import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  claimWords,
  type ClaimOption,
  type ClaimsOptions,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { MarginNote } from "@/components/margin-note";
import type { Basis } from "@/overview/words";
import { useClaims } from "@/searches";
import { useSession } from "@/session";
import { SearchActions, type Chosen } from "@/tools/act";
import { People } from "@/tools/claims/people";
import { Options, type OptionRow } from "@/tools/options";
import type { ToolSearch } from "@/tools/search";
import { basisOf, heldIn, heldOf } from "@/year/search";
import { BasisSwitch } from "@/year/year";
import { offeredName } from "@/workspace";
import { ToolAbout } from "@/tools/about";

const WORDS = claimWords();

/** The plan as it stands, then every set of claims. */
function rowsOf(found: ClaimsOptions, basis: Basis): OptionRow<ClaimOption>[] {
  return [
    {
      key: found.current,
      cells: [found.current, ...found.current_ages, ...found.baseline[basis]],
      narrow: found.current,
      option: null,
    },
    ...found.options.map((option) => {
      const ages = option.claims.map((claim) => String(claim.age));
      return {
        key: option.key,
        cells: ["", ...ages, ...option.figures[basis]],
        narrow: ages.join(" · "),
        option,
      };
    }),
  ];
}

/** The highlighted claims, as they are taken or written. */
function chosenClaims(
  path: string | null,
  option: ClaimOption,
  added: unknown[],
): Chosen {
  return {
    noun: "these claims",
    question: option.question,
    described:
      "These claims are written as a scenario over this plan, to a file of this name in your workspace.",
    offered: offeredName(path, `claims-${option.key}`),
    take: (document) => document.takeClaims(option.claims, added),
    scenario: (document, out) =>
      document.claimsScenario(out, option.claims, added),
  };
}

/**
 * The SSA Benefits tool: each person's record and estimated benefit with
 * what can be done for them, and every claim age for the household ranked
 * best first, the highlighted claims taken or written as a scenario.
 */
export function ClaimsPage() {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const basis = basisOf(search);
  const held = useMemo(() => heldOf({ held: search.held }), [search.held]);
  const { reading, issues, path } = useSession();
  const isValid = issues.length === 0;
  const plan = useMemo(() => reading.document?.planText() ?? "", [reading]);
  const people = useMemo(
    () => reading.document?.people(held) ?? [],
    [reading, held],
  );
  const found = useClaims(plan, held, isValid);
  const reply = found.data;
  const highlighted =
    reply?.options.find((each) => each.key === search.claim) ??
    reply?.options[0];
  const isCurrent = !found.isFetching && !found.isPlaceholderData;
  const at = Math.min(search.person ?? 0, Math.max(people.length - 1, 0));
  const rows = useMemo(() => reply && rowsOf(reply, basis), [reply, basis]);
  const columns = useMemo(() => reply && ["", ...reply.columns], [reply]);

  const hold = (id: string, isHeld: boolean) => {
    const others = held.filter((each) => each !== id);
    void navigate({
      search: (kept) => ({
        ...kept,
        held: heldIn(isHeld ? [...others, id] : others),
        claim: undefined,
      }),
      replace: true,
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">
            SSA Benefits
          </h1>
          <ToolAbout about={WORDS.about} />
        </div>
        <BasisSwitch />
      </div>
      <section aria-labelledby="claims" className="w-fit max-w-full space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex items-baseline gap-3">
            <h2 id="claims" className="text-lg font-semibold">
              Claim options
            </h2>
            {found.isFetching && (
              <span className="text-muted-foreground text-sm">Searching…</span>
            )}
          </div>
          {reply && highlighted && isValid && !found.error && (
            <SearchActions
              key={highlighted.key}
              chosen={chosenClaims(path, highlighted, reply.added)}
              isCurrent={isCurrent}
            />
          )}
        </div>
        {!isValid ? (
          <p className="text-muted-foreground">
            The claims are searched once the plan&apos;s issues are fixed; the{" "}
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
            label="Claim options"
            columns={columns ?? []}
            rows={rows ?? []}
            narrowFigure={(columns ?? []).indexOf(WORDS.against_plan)}
            highlighted={highlighted}
            highlight={(option) => {
              void navigate({
                search: (kept) => ({ ...kept, claim: option.key }),
                replace: true,
              });
            }}
          />
        )}
      </section>
      <section aria-labelledby="people" className="space-y-3">
        <h2 id="people" className="text-lg font-semibold">
          People
        </h2>
        {people.length === 0 ? (
          <p className="text-muted-foreground">{WORDS.nobody}</p>
        ) : (
          <People
            columns={WORDS.people_columns}
            spelledOut={WORDS.spelled_out}
            people={people}
            at={at}
            highlight={(person) => {
              void navigate({
                search: (kept) => ({ ...kept, person }),
                replace: true,
              });
            }}
            hold={hold}
          />
        )}
      </section>
    </div>
  );
}
