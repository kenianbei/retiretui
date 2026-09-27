import { Link, useSearch } from "@tanstack/react-router";
import {
  issueCount,
  type Document,
  type PlacedIssue,
} from "@wasm/retiretui_wasm.js";
import { useMemo, type ReactNode } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { BASIS_LABEL, dollars, share, type Basis } from "@/overview/words";
import { useMonteCarlo } from "@/searches";
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
  return (
    <section
      aria-labelledby="problems"
      className="border-destructive bg-card space-y-3 rounded-md border-l-4 p-4"
    >
      <h2 id="problems" className="text-lg font-semibold">
        This plan has {issueCount(issues.length)}
      </h2>
      <ul className="list-disc space-y-1 pl-5">
        {issues.map((issue) => (
          <li key={issue.words}>{issue.words}</li>
        ))}
      </ul>
      <p className="text-muted-foreground text-sm">
        The figures appear once the plan file is fixed and uploaded again.
      </p>
    </section>
  );
}

/** A headline figure: a card with its label over what it shows. */
function Figure({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Card className="gap-1 py-4">
      <CardHeader className="px-4">
        <CardTitle className="text-muted-foreground text-sm font-normal">
          {label}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-1 px-4">{children}</CardContent>
    </Card>
  );
}

/** A figure's big line, and the small one saying what it is in. */
function Reading({
  big,
  small,
  isShortfall,
}: {
  big: ReactNode;
  small: ReactNode;
  isShortfall?: boolean;
}) {
  return (
    <>
      <p
        className={cn(
          "text-2xl font-semibold tabular-nums",
          isShortfall && "text-destructive",
        )}
      >
        {big}
      </p>
      <p className="text-muted-foreground text-xs">{small}</p>
    </>
  );
}

function Success({ plan }: { plan: string }) {
  const markets = useMonteCarlo(plan);
  if (markets.data) {
    const runs = markets.data.runs.toLocaleString("en-US");
    return (
      <Reading
        big={share(markets.data.success_rate)}
        small={`of ${runs} markets`}
      />
    );
  }
  if (markets.error) {
    return (
      <p className="text-muted-foreground text-sm">{markets.error.message}</p>
    );
  }
  return (
    <>
      <Skeleton className="h-8 w-20" />
      <p className="text-muted-foreground text-xs">Running markets…</p>
    </>
  );
}

function Figures({ document, basis }: { document: Document; basis: Basis }) {
  const summary = useMemo(
    () => document.summary(basis === "today"),
    [document, basis],
  );
  const plan = useMemo(() => document.planText(), [document]);
  if (!summary) return null;
  const unit = BASIS_LABEL[basis];
  const firstShort = summary.first_unfunded_year;
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <Figure label="Lasts through random markets">
        <Success plan={plan} />
      </Figure>
      <Figure label="Net worth at the end">
        <Reading big={dollars(summary.final_net_worth)} small={unit} />
      </Figure>
      <Figure label="Peak net worth">
        <Reading
          big={dollars(summary.peak_net_worth)}
          small={`${unit} · in ${String(summary.peak_year)}`}
        />
      </Figure>
      <Figure label="Spending the money cannot cover">
        <Reading
          big={dollars(summary.lifetime_unfunded)}
          isShortfall={firstShort !== null}
          small={
            firstShort === null
              ? `${unit} · every year is covered`
              : `${unit} · first short in ${String(firstShort)}`
          }
        />
      </Figure>
      <Figure label="Lifetime taxes">
        <Reading big={dollars(summary.lifetime_taxes)} small={unit} />
      </Figure>
      <Figure label="Lifetime Roth conversions">
        <Reading big={dollars(summary.lifetime_conversions)} small={unit} />
      </Figure>
    </div>
  );
}

function ThisYear({ document }: { document: Document }) {
  const said = useMemo(() => {
    const year = document.thisYear(new Date().getFullYear());
    return year === undefined ? null : document.said(year);
  }, [document]);
  if (!said) return null;
  const ages = said.ages.map(([name, age]) => `${name} turns ${String(age)}`);

  return (
    <section aria-labelledby="this-year" className="space-y-3">
      <div className="space-y-1">
        <h2 id="this-year" className="text-lg font-semibold">
          What to do in {said.year}
        </h2>
        <p className="text-muted-foreground text-sm">
          {[...ages, "amounts in nominal $"].join(" · ")}
        </p>
      </div>
      {said.actions.length === 0 ? (
        <p className="text-muted-foreground">Nothing to do this year.</p>
      ) : (
        <ul className="bg-card divide-y rounded-md border tabular-nums">
          {said.actions.map((action, at) => (
            <li key={at} className="px-4 py-3">
              {action}
            </li>
          ))}
        </ul>
      )}
      {said.warnings.length > 0 && (
        <ul className="border-warning space-y-1 rounded-md border-l-4 px-4 py-3">
          {said.warnings.map((warning) => (
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
  const issues = useMemo(() => document?.issues() ?? [], [document]);
  if (!document) return null;

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
