import { useNavigate, useSearch } from "@tanstack/react-router";
import { useCallback, useMemo } from "react";

import { useSession } from "@/session";
import type { YearSearch } from "@/year/search";

/** The year shown and how to show another; no year while nothing is projected. */
export interface ShownYear {
  year: number | undefined;
  /** The years either side, held within the plan; the year itself at its ends. */
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
    return year === undefined
      ? { year, before: undefined, after: undefined }
      : { year, before: at(year - 1), after: at(year + 1) };
  }, [reading, search.year]);
  const setYear = useCallback(
    (year: number) => {
      void navigate({
        to: ".",
        search: (prev) => ({ ...prev, year }),
        replace: true,
      });
    },
    [navigate],
  );
  return { ...shown, setYear };
}
