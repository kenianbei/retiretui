import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  marketWords,
  type AssumptionRow,
  type RunRow,
} from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { placeSearch } from "@/plan/search";
import { useMarkets, type MarketKind } from "@/searches";
import { useSession } from "@/session";
import { MarketCharts } from "@/tools/markets/charts";
import { ZONE_CLASS } from "@/tools/markets/zone";
import { Options, type OptionRow } from "@/tools/options";
import type { ToolSearch } from "@/tools/search";
import { keptSearch } from "@/year/search";

const WORDS = marketWords();
/** The runs table's column a phone's row shows beside its name. */
const ENDS_WITH = 1;

/** An assumption, linked to where it is edited: its field, or its page. */
function AssumptionLink({ row }: { row: AssumptionRow }) {
  const place = placeSearch({
    domain: row.domain,
    index: null,
    field: row.field,
  });
  return (
    <Link
      to="/plan/$page"
      params={{ page: row.domain }}
      search={row.field === null ? {} : place.search}
      className="underline-offset-4 hover:underline"
    >
      {row.value}
    </Link>
  );
}

/** What the runs were made under, each leading to where it is edited. */
function Assumptions({ rows }: { rows: AssumptionRow[] }) {
  return (
    <Card className="gap-2 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-base">Assumptions</CardTitle>
      </CardHeader>
      <CardContent className="px-4">
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
          {rows.map((row) => (
            <div key={row.label} className="contents">
              <dt className="text-muted-foreground">{row.label}</dt>
              <dd className="tabular-nums">
                <AssumptionLink row={row} />
              </dd>
            </div>
          ))}
        </dl>
      </CardContent>
    </Card>
  );
}

/** The highlighted run opened in the Ledger; the plan's own row, the plain Ledger. */
function OpenInLedger({ run }: { run: RunRow }) {
  return (
    <Button asChild>
      <Link
        to="/ledger"
        search={(kept) => ({
          ...keptSearch(kept, ["basis", "held"]),
          ...(run.market !== null && { market: run.market }),
        })}
      >
        Open in Ledger
      </Link>
    </Button>
  );
}

/**
 * A market tool: the plan through `kind`'s markets, how it fared, the runs
 * singled out beside what they were made under, their spread, and the
 * highlighted run opened in the Ledger.
 */
export function MarketsPage({
  kind,
  title,
}: {
  kind: MarketKind;
  title: string;
}) {
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const { reading, issues } = useSession();
  const isValid = issues.length === 0;
  const plan = useMemo(() => reading.document?.planText() ?? "", [reading]);
  const found = useMarkets(kind, plan, isValid);
  const reply = found.data;
  const highlighted =
    reply?.runs.find((run) => run.key === search.run) ?? reply?.runs[0];
  const rows = useMemo<OptionRow<RunRow>[]>(
    () =>
      reply?.runs.map((run) => ({
        key: run.key,
        cells: run.cells,
        narrow: run.cells[0] ?? "",
        option: run,
      })) ?? [],
    [reply],
  );

  return (
    <div className="max-w-5xl space-y-6">
      <div className="space-y-1">
        <div className="flex flex-wrap items-baseline gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          {found.isFetching && (
            <span className="text-muted-foreground text-sm">Searching…</span>
          )}
        </div>
        {reply && (
          <p
            className={cn(
              "text-lg font-semibold",
              ZONE_CLASS[reply.zone],
            )}
          >
            {reply.verdict}
          </p>
        )}
      </div>
      {!isValid ? (
        <p className="text-muted-foreground">
          The markets are run once the plan&apos;s issues are fixed; the{" "}
          <Link to="/overview" className="underline underline-offset-4">
            Overview
          </Link>{" "}
          lists them.
        </p>
      ) : found.error && !found.isFetching ? (
        <Alert>
          <AlertDescription>{found.error.message}</AlertDescription>
        </Alert>
      ) : !reply || !highlighted ? (
        <p className="text-muted-foreground">{WORDS.nothing_searched}</p>
      ) : (
        <>
          <div className="grid items-start gap-4 md:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
            <Assumptions rows={reply.assumptions} />
            <div className="space-y-3">
              <Options
                label={reply.columns[0]}
                columns={reply.columns}
                rows={rows}
                narrowFigure={ENDS_WITH}
                highlighted={highlighted}
                highlight={(run) => {
                  void navigate({
                    search: (kept) => ({ ...kept, run: run.key }),
                    replace: true,
                  });
                }}
              />
              <OpenInLedger run={highlighted} />
            </div>
          </div>
          <MarketCharts found={reply} run={highlighted} />
        </>
      )}
    </div>
  );
}
