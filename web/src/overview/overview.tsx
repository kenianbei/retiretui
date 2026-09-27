import type { PlacedIssue } from "@bindings";
import { Link, useSearch } from "@tanstack/react-router";
import type { Document } from "@wasm/retiretui_wasm.js";
import { useMemo, type ReactNode } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  BASIS_LABEL,
  actionsYear,
  dollars,
  share,
  step,
  type Basis,
} from "@/overview/words";
import { useSearch as useWorkerSearch } from "@/searches";
import { useSession } from "@/session";

function BasisSwitch({ basis }: { basis: Basis }) {
  return (
    <div
      role="group"
      aria-label="Show dollars as"
      className="inline-flex rounded-md border p-0.5 text-sm"
    >
      {(["today", "nominal"] as const).map((each) => (
        <Link
          key={each}
          to="/overview"
          search={{ basis: each }}
          aria-current={each === basis ? "true" : undefined}
          className={cn(
            "rounded px-3 py-1",
            each === basis && "bg-primary text-primary-foreground",
          )}
        >
          {BASIS_LABEL[each]}
        </Link>
      ))}
    </div>
  );
}

function Problems({ issues }: { issues: PlacedIssue[] }) {
  const count =
    issues.length === 1 ? "1 problem" : `${String(issues.length)} problems`;
  return (
    <section
      aria-labelledby="problems"
      className="border-destructive bg-card space-y-3 rounded-md border-l-4 p-4"
    >
      <h2 id="problems" className="text-lg font-semibold">
        This plan has {count}
      </h2>
      <ul className="list-disc space-y-1 pl-5">
        {issues.map((issue) => (
          <li key={`${issue.path}:${issue.message}`}>{issue.words}</li>
        ))}
      </ul>
      <p className="text-muted-foreground text-sm">
        The figures appear once the plan file is fixed and uploaded again.
      </p>
    </section>
  );
}

interface FigureProps {
  label: string;
  amount: number;
  basis: Basis;
  note?: ReactNode;
  isShortfall?: boolean;
}

function Figure({ label, amount, basis, note, isShortfall }: FigureProps) {
  return (
    <Card className="gap-1 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-muted-foreground text-sm font-normal">
          {label}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-1 px-4">
        <p
          className={cn(
            "text-2xl font-semibold tabular-nums",
            isShortfall && "text-destructive",
          )}
        >
          {dollars(amount)}
        </p>
        <p className="text-muted-foreground text-xs">
          {BASIS_LABEL[basis]}
          {note && <> · {note}</>}
        </p>
      </CardContent>
    </Card>
  );
}

function Success({ plan }: { plan: string }) {
  const search = useWorkerSearch("monteCarlo", plan);
  return (
    <Card className="gap-1 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-muted-foreground text-sm font-normal">
          Lasts through random markets
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-1 px-4">
        {search.data ? (
          <>
            <p className="text-2xl font-semibold tabular-nums">
              {share(search.data.success_rate)}
            </p>
            <p className="text-muted-foreground text-xs">
              of {search.data.runs.toLocaleString("en-US")} markets
            </p>
          </>
        ) : search.error ? (
          <p className="text-muted-foreground text-sm">
            {search.error.message}
          </p>
        ) : (
          <>
            <Skeleton className="h-8 w-20" />
            <p className="text-muted-foreground text-xs">Running markets…</p>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function Figures({ document, basis }: { document: Document; basis: Basis }) {
  const summary = document.summary(basis === "today");
  const plan = useMemo(() => document.planText(), [document]);
  if (!summary) return null;
  const isShort = summary.first_unfunded_year !== null;
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <Success plan={plan} />
      <Figure
        label="Net worth at the end"
        amount={summary.final_net_worth}
        basis={basis}
      />
      <Figure
        label="Peak net worth"
        amount={summary.peak_net_worth}
        basis={basis}
        note={`in ${String(summary.peak_year)}`}
      />
      <Figure
        label="Spending the money cannot cover"
        amount={summary.lifetime_unfunded}
        basis={basis}
        isShortfall={isShort}
        note={
          isShort
            ? `first short in ${String(summary.first_unfunded_year)}`
            : "every year is covered"
        }
      />
      <Figure
        label="Lifetime taxes"
        amount={summary.lifetime_taxes}
        basis={basis}
      />
      <Figure
        label="Lifetime Roth conversions"
        amount={summary.lifetime_conversions}
        basis={basis}
      />
    </div>
  );
}

function ThisYear({ document }: { document: Document }) {
  const year = useMemo(() => {
    const years = document.projection()?.years ?? [];
    const first = years[0]?.year ?? 0;
    const last = years.at(-1)?.year ?? first;
    return actionsYear(first, last, new Date().getFullYear());
  }, [document]);
  const reply = document.actions(year);
  const names = document.names();
  const ages = Object.entries(reply.ages).map(
    ([id, age]) => `${names.people[id] ?? id} turns ${String(age)}`,
  );

  return (
    <section aria-labelledby="this-year" className="space-y-3">
      <div className="space-y-1">
        <h2 id="this-year" className="text-lg font-semibold">
          What to do in {year}
        </h2>
        <p className="text-muted-foreground text-sm">
          {[...ages, "amounts in nominal $"].join(" · ")}
        </p>
      </div>
      {reply.actions.length === 0 ? (
        <p className="text-muted-foreground">Nothing to do this year.</p>
      ) : (
        <ul className="bg-card divide-y rounded-md border">
          {reply.actions.map((action, at) => {
            const { verb, amount, detail } = step(action, names);
            return (
              <li
                key={at}
                className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 px-4 py-3"
              >
                <span>
                  <span className="font-medium">{verb}</span>{" "}
                  <span className="text-muted-foreground">{detail}</span>
                </span>
                <span className="font-medium tabular-nums">
                  {dollars(amount)}
                </span>
              </li>
            );
          })}
        </ul>
      )}
      {reply.warnings.length > 0 && (
        <ul className="border-warning space-y-1 rounded-md border-l-4 px-4 py-3">
          {reply.warnings.map((warning) => (
            <li key={warning} className="text-warning">
              {warning}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Whether the money lasts and how surely, and what to do this year. */
export function Overview() {
  const { document } = useSession();
  const { basis } = useSearch({ from: "/overview" });
  if (!document) return null;
  const issues = document.issues();

  return (
    <div className="max-w-5xl space-y-8">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
        {issues.length === 0 && <BasisSwitch basis={basis} />}
      </div>
      {issues.length > 0 ? (
        <Problems issues={issues} />
      ) : (
        <>
          <Figures document={document} basis={basis} />
          <ThisYear document={document} />
        </>
      )}
    </div>
  );
}
