import type { AccountFlows, HistoryChart, Year } from "@wasm/retiretui_wasm.js";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { MarginNote } from "@/components/margin-note";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { MoneyCards } from "@/ledger/funds";
import { VIEW_WORDS } from "@/overview/view-words";

/** One line of an account's year: its figures on the first, a move on each. */
interface FlowLine {
  key: string;
  account: string;
  open: string;
  moved: string;
  growth: string;
  close: string;
}

/** An account's figures beside its first move, then a line per further move. */
function linesOf(flows: AccountFlows): FlowLine[] {
  const growth = [flows.growth, flows.growth_rate].filter(Boolean).join(" · ");
  const count = Math.max(flows.moves.length, 1);
  return Array.from({ length: count }, (_, at) => ({
    key: `${flows.account}:${String(at)}`,
    account: at === 0 ? flows.account : "",
    open: at === 0 ? flows.open : "",
    moved: flows.moves[at] ?? "",
    growth: at === 0 ? growth : "",
    close: at === 0 ? flows.close : "",
  }));
}

const column = columnsFor<FlowLine>();
/** Each flows column's key, in the order the client heads them. */
const FLOW_KEYS = ["account", "open", "moved", "growth", "close"] as const;
const FLOW_COLUMNS = VIEW_WORDS.flow_columns.map(([header, isNumeric], at) => {
  const key = FLOW_KEYS[at] ?? "account";
  return column.display({
    id: key,
    header,
    meta: { isNumeric },
    cell: ({ row }) =>
      key === "moved" ? (
        <span className="block min-w-36 whitespace-normal">
          {row.original.moved}
        </span>
      ) : (
        row.original[key]
      ),
  });
});

/** The year's own card: its milestones, what to do, what to watch, and how far the plan has come. */
function YearCard({ detail, unit }: { detail: Year; unit: string }) {
  const title = [String(detail.year), detail.ages, unit].filter(Boolean);
  return (
    <Card className="gap-3 py-4">
      <CardHeader className="px-4">
        <CardTitle>
          <h2>{title.join(" · ")}</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 px-4 text-sm">
        {detail.milestones.length > 0 && (
          <ul aria-label={VIEW_WORDS.milestones} className="space-y-1">
            {detail.milestones.map((milestone) => (
              <li key={milestone} className="font-medium">
                <span aria-hidden className="text-primary">
                  ◆{" "}
                </span>
                {milestone}
              </li>
            ))}
          </ul>
        )}
        <ul aria-label={VIEW_WORDS.to_do} className="space-y-1 tabular-nums">
          {detail.to_do.map((action) => (
            <li key={action}>{action}</li>
          ))}
        </ul>
        {detail.warnings.length > 0 && (
          <MarginNote zone="caution">
            <ul aria-label="To watch" className="space-y-1">
              {detail.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </MarginNote>
        )}
        {detail.so_far !== null && (
          <p className="text-muted-foreground tabular-nums">{detail.so_far}</p>
        )}
      </CardContent>
    </Card>
  );
}

/** One year in full: the year itself, its flows through each account, and its money and tax over their histories. */
export function YearCards({
  detail,
  unit,
  histories,
  onYear,
}: {
  detail: Year;
  unit: string;
  histories: readonly HistoryChart[];
  onYear: (year: number) => void;
}) {
  const accounts = [
    ...detail.flows,
    ...(detail.all_accounts ? [detail.all_accounts] : []),
  ];
  const flows = `${String(detail.year)} ${VIEW_WORDS.flows}`;
  return (
    <div className="@container min-w-0 space-y-4">
      <YearCard detail={detail} unit={unit} />
      <Card className="gap-3 py-4">
        <CardHeader className="px-4">
          <CardTitle>
            <h3>{VIEW_WORDS.flows}</h3>
          </CardTitle>
        </CardHeader>
        <CardContent className="px-4">
          <DataTable
            label={flows}
            columns={FLOW_COLUMNS}
            rows={accounts.flatMap(linesOf)}
            rowKey={(line) => line.key}
            isFirstPinned
            isFlush
            className="-mx-4"
          />
        </CardContent>
      </Card>
      <MoneyCards detail={detail} histories={histories} onYear={onYear} />
    </div>
  );
}
