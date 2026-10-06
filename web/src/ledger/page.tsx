import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useMemo, useRef } from "react";

import { MarginNote } from "@/components/margin-note";
import { columnSetOf } from "@/ledger/columns";
import { ColumnsPick, MarkedStepper, ViewSwitch } from "@/ledger/controls";
import { YearCards } from "@/ledger/year-detail";
import { YearsList, YearTable } from "@/ledger/years";
import { messageOf } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { basisOf, type LedgerSearch } from "@/year/search";
import { useYear } from "@/year/use-year";
import { BasisSwitch, YearStepper } from "@/year/year";

/** The Ledger in the plan's own market, the year and basis kept. */
function BackToPlan() {
  return (
    <Link
      to="/ledger"
      search={(kept) => ({ ...kept, market: undefined })}
      className="underline underline-offset-4"
    >
      Back to the plan
    </Link>
  );
}

/** The page under its heading alone, saying why it shows nothing. */
function Unshown({ children }: { children: React.ReactNode }) {
  return (
    <div className="max-w-3xl space-y-3">
      <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
      <p className="text-muted-foreground">{children}</p>
    </div>
  );
}

/** What the Ledger reads of the document for the address it is at: every year, and then the year shown. */
function useLedger(search: LedgerSearch, year: number | undefined) {
  const { reading } = useSession();
  const { market } = search;
  const isNominal = basisOf(search) === "nominal";
  const set = columnSetOf(search.columns);
  const isTable = search.view === "table";
  const plan = useMemo(() => {
    const document = reading.document;
    try {
      const ledger = document?.ledger(isNominal, market, set);
      return {
        ledger,
        said: market === undefined ? "" : document?.marketSaid(market),
        histories:
          ledger && !isTable
            ? document?.ledgerHistories(isNominal, market)
            : undefined,
      };
    } catch (thrown) {
      return { refusal: messageOf(thrown), ledger: undefined };
    }
  }, [reading, isNominal, market, set, isTable]);
  const shown = useMemo(() => {
    const document = reading.document;
    const isShown = plan.ledger && year !== undefined;
    return {
      detail:
        isShown && !isTable
          ? document?.ledgerYear(year, isNominal, market)
          : undefined,
      marked: isShown ? document?.markedYears(year, market) : undefined,
    };
  }, [reading, plan, isNominal, market, isTable, year]);
  return { ...plan, ...shown };
}

/** One year in the context of all of them, or every year in one table. */
export function LedgerPage() {
  const { reading, issues } = useSession();
  const search: LedgerSearch = useSearch({ from: "/ledger" });
  const navigate = useNavigate();
  const shown = useYear();
  const { year, setYear } = shown;
  const read = useLedger(search, year);
  const isTable = search.view === "table";
  const years = useRef<HTMLDivElement>(null);
  useEffect(() => {
    years.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [year, isTable]);

  if (!reading.document) return null;
  if (read.refusal !== undefined) {
    return (
      <Unshown>
        {read.refusal}. <BackToPlan />
      </Unshown>
    );
  }
  const { ledger, detail } = read;
  if (!ledger) {
    return (
      <Unshown>
        The ledger shows once the plan&apos;s issues are fixed; the{" "}
        <Link to="/overview" className="underline underline-offset-4">
          Overview
        </Link>{" "}
        lists them.
      </Unshown>
    );
  }
  const unit = BASIS_LABEL[basisOf(search)];
  const showYear = (picked: number) => {
    void navigate({
      to: ".",
      search: (prev) => ({ ...prev, year: picked, view: undefined }),
      replace: true,
    });
  };
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <div className="flex flex-wrap items-center gap-2">
          <YearStepper shown={shown} />
          <MarkedStepper
            marked={read.marked ?? [null, null]}
            setYear={setYear}
          />
          <BasisSwitch />
          <ViewSwitch />
        </div>
      </div>
      {search.market !== undefined && (
        <MarginNote zone="note">
          <p className="text-sm first-letter:uppercase">
            {read.said}: the plan as a market tool ran it. <BackToPlan />
          </p>
        </MarginNote>
      )}
      {issues.length > 0 && (
        <MarginNote zone="shortfall">
          <p className="text-muted-foreground text-sm">
            These are the last figures the plan had without issues.
          </p>
        </MarginNote>
      )}
      {isTable ? (
        <div ref={years} className="min-w-0 space-y-3">
          <ColumnsPick set={columnSetOf(search.columns)} />
          <YearTable
            ledger={ledger}
            unit={unit}
            year={year}
            onSelect={showYear}
          />
        </div>
      ) : (
        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[18rem_minmax(0,1fr)]">
          <div ref={years} className="sticky top-4 min-w-0 max-lg:hidden">
            <YearsList
              ledger={ledger}
              unit={unit}
              year={year}
              onSelect={setYear}
            />
          </div>
          {detail && (
            <YearCards
              detail={detail}
              unit={unit}
              histories={read.histories ?? []}
              onYear={setYear}
            />
          )}
        </div>
      )}
    </div>
  );
}
