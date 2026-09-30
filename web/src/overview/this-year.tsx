import { useMemo } from "react";

import { MarginNote } from "@/components/margin-note";
import type { Basis } from "@/overview/words";
import { BASIS_LABEL, VIEW_WORDS } from "@/overview/view-words";
import { Rows } from "@/overview/lists";
import { useSession } from "@/session";
import { YearInLedger } from "@/year/ledger-link";
import type { ShownYear } from "@/year/use-year";
import { YearStepper } from "@/year/year";

/** What to do in the year shown, and what to watch, in the dollars shown. */
export function ThisYear({ shown, basis }: { shown: ShownYear; basis: Basis }) {
  const { reading } = useSession();
  const { year } = shown;
  const said = useMemo(
    () =>
      year === undefined
        ? null
        : reading.document?.said(year, basis === "nominal"),
    [reading, year, basis],
  );
  if (!said) return null;
  const ages = said.ages.map(([name, age]) => `${name} turns ${String(age)}`);

  return (
    <section aria-labelledby="this-year" className="max-w-3xl space-y-3">
      <div className="space-y-1">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 id="this-year" className="text-lg font-semibold">
            What to do in {said.year}
          </h2>
          <YearStepper shown={shown} />
        </div>
        <p className="text-muted-foreground flex flex-wrap gap-x-3 text-sm">
          <span>{[...ages, BASIS_LABEL[basis]].join(" · ")}</span>
          <YearInLedger
            year={said.year}
            className="text-primary underline-offset-4 hover:underline"
          >
            {said.year} in the Ledger
          </YearInLedger>
        </p>
      </div>
      {said.actions.length === 0 ? (
        <p className="text-muted-foreground text-sm">
          {VIEW_WORDS.nothing_scheduled}
        </p>
      ) : (
        <Rows>
          {said.actions.map((action, at) => (
            <li key={at} className="px-4 py-2.5 text-sm tabular-nums">
              {action}
            </li>
          ))}
        </Rows>
      )}
      {said.warnings.length > 0 && (
        <MarginNote zone="caution">
          <ul className="space-y-1 text-sm">
            {said.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </MarginNote>
      )}
    </section>
  );
}
