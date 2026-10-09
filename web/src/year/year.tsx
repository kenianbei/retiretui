import { Link, useSearch } from "@tanstack/react-router";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { type ReactNode, useEffect } from "react";

import { Button } from "@/components/ui/button";
import { isHeld } from "@/lib/keys";
import { cn } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { basisIn, basisOf, IN_PLACE, type YearSearch } from "@/year/search";
import type { ShownYear } from "@/year/use-year";

/** ← and → step the year wherever nothing else holds the key. */
function useYearKeys({ before, after, setYear }: ShownYear) {
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) {
        return;
      }
      const target = event.target;
      if (
        isHeld(target) ||
        (target instanceof HTMLElement &&
          target.closest('[role="tablist"]') !== null)
      ) {
        return;
      }
      const next =
        event.key === "ArrowLeft"
          ? before
          : event.key === "ArrowRight"
            ? after
            : undefined;
      if (next === undefined) return;
      event.preventDefault();
      setYear(next);
    };
    window.addEventListener("keydown", press);
    return () => {
      window.removeEventListener("keydown", press);
    };
  }, [before, after, setYear]);
}

/** A button that shows the year `to`, and does nothing where there is none. */
export function StepButton({
  label,
  to,
  setYear,
  children,
}: {
  label: string;
  to: number | null | undefined;
  setYear: (year: number) => void;
  children: ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      disabled={to == null}
      onClick={() => {
        if (to != null) setYear(to);
      }}
    >
      {children}
    </Button>
  );
}

/** The year shown between the years either side; ← and → step it too. */
export function YearStepper({ shown }: { shown: ShownYear }) {
  useYearKeys(shown);
  const { year, before, after, setYear } = shown;
  if (year === undefined) return null;
  return (
    <div role="group" aria-label="Year" className="inline-flex items-center">
      <StepButton label="The year before" to={before} setYear={setYear}>
        <ChevronLeft aria-hidden />
      </StepButton>
      <span aria-live="polite" className="w-12 text-center tabular-nums">
        {year}
      </span>
      <StepButton label="The year after" to={after} setYear={setYear}>
        <ChevronRight aria-hidden />
      </StepButton>
    </div>
  );
}

/** One of a few choices kept in the address. */
interface Choice {
  label: string;
  isCurrent: boolean;
  /** The address with the choice made, from the one shown. */
  search: (prev: Record<string, unknown>) => Record<string, unknown>;
}

/** A few choices side by side, the one made marked, each a link that keeps the page and the rest of its address. */
export function Segmented({
  label,
  choices,
}: {
  label: string;
  choices: readonly Choice[];
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className="inline-flex rounded-md border p-0.5 text-sm"
    >
      {choices.map((choice) => (
        <Link
          key={choice.label}
          to="."
          search={choice.search}
          {...IN_PLACE}
          aria-current={choice.isCurrent ? "true" : undefined}
          className={cn(
            "max-md:touch-target relative rounded px-3 py-1",
            choice.isCurrent && "bg-primary text-primary-foreground",
          )}
        >
          {choice.label}
        </Link>
      ))}
    </div>
  );
}

/** Today's dollars or nominal, keeping the year and the page. */
export function BasisSwitch() {
  const search: YearSearch = useSearch({ strict: false });
  const basis = basisOf(search);
  return (
    <Segmented
      label="Show dollars as"
      choices={(["today", "nominal"] as const).map((each) => ({
        label: BASIS_LABEL[each],
        isCurrent: each === basis,
        search: (prev) => ({ ...prev, basis: basisIn(each) }),
      }))}
    />
  );
}
