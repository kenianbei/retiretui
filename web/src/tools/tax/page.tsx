import { useNavigate, useSearch } from "@tanstack/react-router";
import type { Offer, TaxSection } from "@wasm/retiretui_wasm.js";
import { useMemo } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn, INPUT } from "@/lib/utils";
import { useSession } from "@/session";
import { ToolAbout } from "@/tools/about";
import type { ToolSearch } from "@/tools/search";
import { useYear } from "@/year/use-year";
import { YearStepper } from "@/year/year";

/** What a picker offers before any choice of its own: the plan's. */
const THE_PLAN_S = "";

interface PickerProps {
  label: string;
  /** What the plan itself is shown under, where nothing is picked. */
  own: string;
  value: string | undefined;
  choices: Offer[];
  pick: (key: string | undefined) => void;
}

/** A choice of status or state, the plan's own first. */
function Picker({ label, own, value, choices, pick }: PickerProps) {
  return (
    <label className="flex min-w-0 items-center gap-2 text-sm">
      <span className="text-muted-foreground whitespace-nowrap">{label}</span>
      <select
        value={value ?? THE_PLAN_S}
        onChange={(event) => {
          const key = event.target.value;
          pick(key === THE_PLAN_S ? undefined : key);
        }}
        className={cn(INPUT, "h-8 w-auto min-w-0")}
      >
        <option value={THE_PLAN_S}>{own}</option>
        {choices.map((choice) => (
          <option key={choice.value} value={choice.value}>
            {choice.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Past this many rows, a table of a number and its figure is set as pairs in columns. */
const LONG = 12;
const NUMBER = /^\d+$/;

function isPairs(section: TaxSection): boolean {
  return (
    section.columns.length === 2 &&
    section.rows.length > LONG &&
    section.rows.every(([first]) => NUMBER.test(first ?? ""))
  );
}

/** A long table of two columns - an age and its divisor - as pairs that flow. */
function Pairs({ section }: { section: TaxSection }) {
  const [label, figure] = section.columns;
  return (
    <div className="@container space-y-2 text-sm">
      <p className="text-muted-foreground">
        {label} · {figure}
      </p>
      <dl className="columns-2 gap-x-6 @xs:columns-3 @md:columns-4">
        {section.rows.map(([first, second]) => (
          <div
            key={first}
            className="flex break-inside-avoid justify-between gap-2 py-0.5"
          >
            <dt>{first}</dt>
            <dd className="tabular-nums">{second}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** One of the year's tables, or why it has no rows. */
function Section({ section }: { section: TaxSection }) {
  const [label, ...figures] = section.columns;
  return (
    <Card className="mb-4 break-inside-avoid gap-2 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-base">
          <h2>{section.title}</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="px-4">
        {section.note !== null ? (
          <p className="text-muted-foreground text-sm">{section.note}</p>
        ) : isPairs(section) ? (
          <Pairs section={section} />
        ) : (
          <table className="w-full text-sm">
            <thead className="text-muted-foreground text-left">
              <tr>
                <th scope="col" className="py-1 font-medium">
                  {label}
                </th>
                {figures.map((column) => (
                  <th
                    key={column}
                    scope="col"
                    className="py-1 text-right font-medium"
                  >
                    {column}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y">
              {section.rows.map(([first, ...rest]) => (
                <tr key={first}>
                  <th scope="row" className="py-1.5 pr-4 text-left font-normal">
                    {first}
                  </th>
                  {rest.map((cell, at) => (
                    <td key={at} className="py-1.5 text-right tabular-nums">
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * The tables the plan's projection applies in the year: its filing
 * status's and the state it lives in, or any other picked.
 */
export function TaxTablesPage({ title }: { title: string }) {
  const { reading } = useSession();
  const search: ToolSearch = useSearch({ from: "/tools/$page" });
  const navigate = useNavigate({ from: "/tools/$page" });
  const shown = useYear();
  const year = shown.year ?? search.year ?? new Date().getFullYear();
  const tables = useMemo(
    () =>
      reading.document?.taxTables({
        year,
        status: search.status,
        state: search.state,
      }),
    [reading, year, search.status, search.state],
  );
  if (!tables) return null;
  const place = (picked: Pick<ToolSearch, "status" | "state">) => {
    void navigate({
      search: (prev) => ({ ...prev, ...picked }),
      replace: true,
    });
  };
  const named = (choices: Offer[], key: string | null) =>
    choices.find((choice) => choice.value === key)?.label ?? "none";

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          <ToolAbout about={tables.about} />
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <YearStepper shown={shown} />
          <Picker
            label="Filing status"
            own={`The plan's (${named(tables.statuses, tables.status)})`}
            value={search.status}
            choices={tables.statuses}
            pick={(status) => {
              place({ status });
            }}
          />
          <Picker
            label="State"
            own={
              search.state === undefined
                ? `Where the plan lives (${named(tables.states, tables.state)})`
                : "Where the plan lives"
            }
            value={search.state}
            choices={tables.states}
            pick={(state) => {
              place({ state });
            }}
          />
        </div>
      </div>
      <div className="gap-x-4 @3xl/page:columns-2 @7xl/page:columns-3">
        {tables.sections.map((section) => (
          <Section key={section.title} section={section} />
        ))}
      </div>
    </div>
  );
}
