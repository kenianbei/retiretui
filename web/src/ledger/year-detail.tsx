import type {
  AccountFlows,
  DetailLine,
  YearDetail,
} from "@wasm/retiretui_wasm.js";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { MarginNote } from "@/components/margin-note";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { VIEW_WORDS } from "@/overview/view-words";

/** One line of an account's year: its figures on the first, a flow on each. */
interface FlowLine {
  key: string;
  account: string;
  open: string;
  came: string;
  went: string;
  growth: string;
  close: string;
}

/** An account's figures beside its first flow in and out, then a line per further flow. */
function linesOf(flows: AccountFlows): FlowLine[] {
  const count = Math.max(flows.ins.length, flows.outs.length, 1);
  return Array.from({ length: count }, (_, at) => ({
    key: `${flows.account}:${String(at)}`,
    account: at === 0 ? flows.account : "",
    open: at === 0 ? flows.open : "",
    came: flows.ins[at] ?? "",
    went: flows.outs[at] ?? "",
    growth: at === 0 ? flows.growth : "",
    close: at === 0 ? flows.close : "",
  }));
}

const column = columnsFor<FlowLine>();
/** The columns naming where money came from or went, which wrap so the figures stay in view. */
const NAMING: readonly (keyof FlowLine)[] = ["came", "went"];

const flowColumn = (key: keyof FlowLine, header: string, isNumeric: boolean) =>
  column.display({
    id: key,
    header,
    meta: { isNumeric },
    cell: ({ row }) =>
      NAMING.includes(key) ? (
        <span className="block min-w-32 whitespace-normal">
          {row.original[key]}
        </span>
      ) : (
        row.original[key]
      ),
  });
/** Each flows column's key, in the order the client heads them. */
const FLOW_KEYS = [
  "account",
  "open",
  "came",
  "went",
  "growth",
  "close",
] as const;
const FLOW_COLUMNS = VIEW_WORDS.flow_headers.map(([header, isFigure], at) =>
  flowColumn(FLOW_KEYS[at] ?? "account", header, isFigure),
);

function Lines({ label, lines }: { label: string; lines: DetailLine[] }) {
  return (
    <dl aria-label={label} className="divide-y text-sm">
      {lines.map((line) => (
        <div key={line.label} className="flex justify-between gap-4 py-1.5">
          <dt>{line.label}</dt>
          <dd className="tabular-nums">{line.amount}</dd>
        </div>
      ))}
    </dl>
  );
}

/** What the year has the household do, its flows through each account, and its income and what it paid. */
export function YearDetailCards({
  year,
  unit,
  detail,
}: {
  year: number;
  unit: string;
  detail: YearDetail;
}) {
  const title = (what: string) => `${String(year)} ${what} · ${unit}`;
  const ages = detail.ages.map(([name, age]) => `${name} turns ${String(age)}`);
  return (
    <>
      <Card className="gap-3 py-4 @4xl:col-span-2">
        <CardHeader className="px-4">
          <CardTitle>{title(VIEW_WORDS.to_do)}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 px-4 text-sm">
          {ages.length > 0 && (
            <p className="text-muted-foreground">{ages.join(" · ")}</p>
          )}
          {detail.actions.length === 0 ? (
            <p className="text-muted-foreground">
              {VIEW_WORDS.nothing_scheduled}
            </p>
          ) : (
            <ul className="space-y-1 tabular-nums">
              {detail.actions.map((action, at) => (
                <li key={at}>{action}</li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
      <Card className="gap-3 py-4">
        <CardHeader className="px-4">
          <CardTitle>{title(VIEW_WORDS.flows)}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 px-4">
          <DataTable
            label={title(VIEW_WORDS.flows)}
            columns={FLOW_COLUMNS}
            rows={detail.flows.flatMap(linesOf)}
            rowKey={(line) => line.key}
            isFirstPinned
            isFlush
            className="-mx-4"
          />
          {detail.warnings.length > 0 && (
            <MarginNote zone="caution">
              <ul className="space-y-1 text-sm">
                {detail.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            </MarginNote>
          )}
        </CardContent>
      </Card>
      <Card className="gap-3 py-4">
        <CardHeader className="px-4">
          <CardTitle>{title(VIEW_WORDS.income_and_tax)}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 px-4">
          {detail.income.length > 0 && (
            <Lines label="Income" lines={detail.income} />
          )}
          <Lines label="Paid" lines={detail.paid} />
        </CardContent>
      </Card>
    </>
  );
}
