import { useSearch } from "@tanstack/react-router";
import { issueCount, type PlacedIssue } from "@wasm/retiretui_wasm.js";
import { useMemo, type ReactNode } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { IssueLink } from "@/draft/issue-link";
import { Better } from "@/overview/better";
import { Charts } from "@/overview/charts";
import { BASIS_LABEL, dollars, share, type Basis } from "@/overview/words";
import { useMonteCarlo } from "@/searches";
import { useSession } from "@/session";
import { basisOf, heldOf } from "@/year/search";
import { useYear, type ShownYear } from "@/year/use-year";
import { BasisSwitch, YearStepper } from "@/year/year";

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
          <li key={issue.words}>
            <IssueLink issue={issue} className="underline underline-offset-4" />
          </li>
        ))}
      </ul>
      <p className="text-muted-foreground text-sm">
        The figures below are the last ones the plan had without issues.
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
    const runs = markets.data.count.toLocaleString("en-US");
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

function Figures({ basis, plan }: { basis: Basis; plan: string | null }) {
  const { reading } = useSession();
  const summary = useMemo(
    () => reading.document?.summary(basis === "today"),
    [reading, basis],
  );
  if (!summary) return null;
  const unit = BASIS_LABEL[basis];
  const firstShort = summary.first_unfunded_year;
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <Figure label="Lasts through random markets">
        {plan !== null ? (
          <Success plan={plan} />
        ) : (
          <Reading big="—" small="once the plan's issues are fixed" />
        )}
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

function ThisYear({ shown }: { shown: ShownYear }) {
  const { reading } = useSession();
  const { year } = shown;
  const said = useMemo(
    () => (year === undefined ? null : reading.document?.said(year)),
    [reading, year],
  );
  if (!said) return null;
  const ages = said.ages.map(([name, age]) => `${name} turns ${String(age)}`);

  return (
    <section aria-labelledby="this-year" className="space-y-3">
      <div className="space-y-1">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 id="this-year" className="text-lg font-semibold">
            What to do in {said.year}
          </h2>
          <YearStepper shown={shown} />
        </div>
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

/** The plan charted year by year, a click choosing the year shown. */
function PlanCharts({
  basis,
  plan,
  shown,
}: {
  basis: Basis;
  plan: string | null;
  shown: ShownYear;
}) {
  const { reading } = useSession();
  const { year, setYear } = shown;
  const series = useMemo(
    () => reading.document?.chart(basis === "nominal"),
    [reading, basis],
  );
  if (!series) return null;
  return (
    <Charts
      series={series}
      basis={basis}
      plan={plan}
      year={year}
      onYear={setYear}
    />
  );
}

/** Whether the money lasts and how surely, the plan charted, and what to do in a year. */
export function Overview() {
  const { reading, document, issues } = useSession();
  const search = useSearch({ from: "/overview" });
  const basis = basisOf(search);
  const held = useMemo(() => heldOf({ held: search.held }), [search.held]);
  const isValid = issues.length === 0;
  const shown = useYear();
  const plan = useMemo(
    () => (isValid ? (reading.document?.planText() ?? null) : null),
    [reading, isValid],
  );
  if (!document) return null;

  return (
    <div className="max-w-5xl space-y-8">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
        <BasisSwitch />
      </div>
      {issues.length > 0 && <Problems issues={issues} />}
      <Figures basis={basis} plan={plan} />
      {plan !== null && <Better plan={plan} held={held} basis={basis} />}
      <PlanCharts basis={basis} plan={plan} shown={shown} />
      <ThisYear shown={shown} />
    </div>
  );
}
