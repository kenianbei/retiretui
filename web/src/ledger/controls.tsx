import { useNavigate, useSearch } from "@tanstack/react-router";
import { ChevronsLeft, ChevronsRight } from "lucide-react";

import { FIRST_SET } from "@/ledger/columns";
import { cn, INPUT } from "@/lib/utils";
import { VIEW_WORDS } from "@/overview/view-words";
import { IN_PLACE, type LedgerSearch } from "@/year/search";
import { Segmented, StepButton } from "@/year/year";

/** The nearest year with a milestone or a warning either side of the one shown. */
export function MarkedStepper({
  marked,
  setYear,
}: {
  marked: readonly [number | undefined, number | undefined];
  setYear: (year: number) => void;
}) {
  const [before, after] = marked;
  return (
    <div
      role="group"
      aria-label="Marked year"
      className="inline-flex items-center"
    >
      <StepButton label="The marked year before" to={before} setYear={setYear}>
        <ChevronsLeft aria-hidden />
      </StepButton>
      <StepButton label="The marked year after" to={after} setYear={setYear}>
        <ChevronsRight aria-hidden />
      </StepButton>
    </div>
  );
}

const VIEWS = [
  ["Year", undefined],
  ["Table", "table"],
] as const;

/** The year in full or the whole year table, keeping everything else in the address. */
export function ViewSwitch() {
  const search: LedgerSearch = useSearch({ from: "/ledger" });
  return (
    <Segmented
      label="Show"
      choices={VIEWS.map(([label, view]) => ({
        label,
        isCurrent: view === search.view,
        search: (prev) => ({ ...prev, view }),
      }))}
    />
  );
}

/** The column set the year table is under. */
export function ColumnsPick({ set }: { set: string | undefined }) {
  const navigate = useNavigate();
  return (
    <label className="flex items-center gap-2 text-sm">
      <span className="text-muted-foreground">Columns</span>
      <select
        value={set}
        onChange={(event) => {
          const picked = event.target.value;
          void navigate({
            to: ".",
            search: (prev) => ({
              ...prev,
              columns: picked === FIRST_SET?.[0] ? undefined : picked,
            }),
            ...IN_PLACE,
          });
        }}
        className={cn(INPUT, "h-8 w-auto")}
      >
        {VIEW_WORDS.column_sets.map(([slug, title]) => (
          <option key={slug} value={slug}>
            {title}
          </option>
        ))}
      </select>
    </label>
  );
}
