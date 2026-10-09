import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useMemo } from "react";

import { MarginNote } from "@/components/margin-note";
import { columnSetOf } from "@/ledger/columns";
import { ColumnsPick, MarkedStepper, ViewSwitch } from "@/ledger/controls";
import { markedBeside } from "@/ledger/marked";
import { YearCards } from "@/ledger/year-detail";
import { Years } from "@/ledger/years";
import { messageOf } from "@/lib/utils";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { basisOf, IN_PLACE, type LedgerSearch } from "@/year/search";
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

/** What the Ledger reads of the document for the address it is at, each reading made again only when what it is of moves. */
function useLedger(search: LedgerSearch, year: number | undefined) {
  // The reading is the dependency, not its document: an edit renews the one and keeps the other.
  const { reading } = useSession();
  const { market } = search;
  const isNominal = basisOf(search) === "nominal";
  const set = columnSetOf(search.columns);
  const isTable = search.view === "table";
  const plan = useMemo(() => {
    const document = reading.document;
    try {
      return {
        ledger: document?.ledger(isNominal, market, set),
        said: market === undefined ? "" : document?.marketSaid(market),
      };
    } catch (thrown) {
      return { refusal: messageOf(thrown), ledger: undefined };
    }
  }, [reading, isNominal, market, set]);
  const isYearShown = Boolean(plan.ledger) && !isTable;
  const detail = useMemo(
    () =>
      isYearShown && year !== undefined
        ? reading.document?.ledgerYear(year, isNominal, market)
        : undefined,
    [reading, isYearShown, year, isNominal, market],
  );
  return { ...plan, detail, set, isTable };
}

/** Every year in a table, over the one shown in full or alone. */
export function LedgerPage() {
  const { reading, issues } = useSession();
  const search: LedgerSearch = useSearch({ from: "/ledger" });
  const navigate = useNavigate();
  const shown = useYear();
  const { year, setYear } = shown;
  const read = useLedger(search, year);
  const { ledger, detail, isTable } = read;
  const marked = useMemo(
    () => markedBeside(ledger?.rows ?? [], year ?? 0),
    [ledger, year],
  );

  if (!reading.document) return null;
  if (read.refusal !== undefined) {
    return (
      <Unshown>
        {read.refusal}. <BackToPlan />
      </Unshown>
    );
  }
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
      ...IN_PLACE,
    });
  };
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Ledger</h1>
        <div className="flex flex-wrap items-center gap-2">
          <YearStepper shown={shown} />
          <MarkedStepper marked={marked} setYear={setYear} />
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
      <div className="min-w-0 space-y-3">
        <ColumnsPick set={read.set} />
        <Years
          ledger={ledger}
          unit={unit}
          year={year}
          onSelect={isTable ? showYear : setYear}
          isAlone={isTable}
        />
      </div>
      {detail && <YearCards detail={detail} />}
    </div>
  );
}
