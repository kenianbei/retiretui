import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { ChevronsLeft, ChevronsRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { FIRST_SET } from "@/ledger/columns";
import { cn, INPUT } from "@/lib/utils";
import { VIEW_WORDS } from "@/overview/view-words";
import type { LedgerSearch } from "@/year/search";

/** The nearest year with a milestone or a warning either side of the one shown. */
export function MarkedStepper({
  marked,
  setYear,
}: {
  marked: readonly [number | null, number | null];
  setYear: (year: number) => void;
}) {
  const [before, after] = marked;
  const step = (to: number | null) =>
    to === null
      ? undefined
      : () => {
          setYear(to);
        };
  return (
    <div
      role="group"
      aria-label="Marked year"
      className="inline-flex items-center"
    >
      <Button
        variant="ghost"
        size="icon"
        aria-label="The marked year before"
        disabled={before === null}
        onClick={step(before)}
      >
        <ChevronsLeft aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label="The marked year after"
        disabled={after === null}
        onClick={step(after)}
      >
        <ChevronsRight aria-hidden />
      </Button>
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
    <div
      role="group"
      aria-label="Show"
      className="inline-flex rounded-md border p-0.5 text-sm"
    >
      {VIEWS.map(([label, view]) => (
        <Link
          key={label}
          to="."
          search={(prev) => ({ ...prev, view })}
          replace
          aria-current={view === search.view ? "true" : undefined}
          className={cn(
            "max-md:touch-target relative rounded px-3 py-1",
            view === search.view && "bg-primary text-primary-foreground",
          )}
        >
          {label}
        </Link>
      ))}
    </div>
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
            replace: true,
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
