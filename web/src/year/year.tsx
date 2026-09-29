import { Link, useSearch } from "@tanstack/react-router";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useEffect } from "react";

import { Button } from "@/components/ui/button";
import { isHeld } from "@/lib/keys";
import { cn } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { basisIn, basisOf, type YearSearch } from "@/year/search";
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

/** The year shown between the years either side; ← and → step it too. */
export function YearStepper({ shown }: { shown: ShownYear }) {
  useYearKeys(shown);
  const { year, before, after, setYear } = shown;
  if (year === undefined) return null;
  const step = (to: number | undefined) =>
    to === undefined
      ? undefined
      : () => {
          setYear(to);
        };
  const earlier = step(before);
  const later = step(after);
  return (
    <div role="group" aria-label="Year" className="inline-flex items-center">
      <Button
        variant="ghost"
        size="icon"
        className="max-md:touch-target"
        aria-label="The year before"
        disabled={!earlier}
        onClick={earlier}
      >
        <ChevronLeft aria-hidden />
      </Button>
      <span aria-live="polite" className="w-12 text-center tabular-nums">
        {year}
      </span>
      <Button
        variant="ghost"
        size="icon"
        className="max-md:touch-target"
        aria-label="The year after"
        disabled={!later}
        onClick={later}
      >
        <ChevronRight aria-hidden />
      </Button>
    </div>
  );
}

/** Today's dollars or nominal, keeping the year and the page. */
export function BasisSwitch() {
  const search: YearSearch = useSearch({ strict: false });
  const basis = basisOf(search);
  return (
    <div
      role="group"
      aria-label="Show dollars as"
      className="inline-flex rounded-md border p-0.5 text-sm"
    >
      {(["today", "nominal"] as const).map((each) => (
        <Link
          key={each}
          to="."
          search={(prev) => ({
            ...prev,
            basis: basisIn(each),
          })}
          replace
          aria-current={each === basis ? "true" : undefined}
          className={cn(
            "max-md:touch-target rounded px-3 py-1",
            each === basis && "bg-primary text-primary-foreground",
          )}
        >
          {BASIS_LABEL[each]}
        </Link>
      ))}
    </div>
  );
}
