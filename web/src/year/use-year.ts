import { useNavigate, useSearch } from "@tanstack/react-router";
import { useCallback, useMemo } from "react";

import { useSession } from "@/session";
import { IN_PLACE, type YearSearch } from "@/year/search";

/** The year shown and how to show another; no year while nothing is projected. */
export interface ShownYear {
  year: number | undefined;
  /** The years either side, where the plan has them. */
  before: number | undefined;
  after: number | undefined;
  setYear: (year: number) => void;
}

export function useYear(): ShownYear {
  const search: YearSearch = useSearch({ strict: false });
  const navigate = useNavigate();
  const { reading } = useSession();
  const shown = useMemo(() => {
    const at = (requested: number | null) =>
      reading.document?.yearAt(requested, new Date().getFullYear());
    const year = at(search.year ?? null);
    if (year === undefined) return { year, before: year, after: year };
    const beside = (step: number) => {
      const next = at(year + step);
      return next === year ? undefined : next;
    };
    return { year, before: beside(-1), after: beside(1) };
  }, [reading, search.year]);
  const setYear = useCallback(
    (year: number) => {
      void navigate({
        to: ".",
        search: (prev) => ({ ...prev, year }),
        ...IN_PLACE,
      });
    },
    [navigate],
  );
  return { ...shown, setYear };
}
