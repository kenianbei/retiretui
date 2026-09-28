import { Link, useNavigate } from "@tanstack/react-router";
import type { FieldView } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useState } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useFileActions } from "@/files/actions";
import { INPUT, messageOf } from "@/lib/utils";
import type { Answering } from "@/onboarding/page";
import { keepAnswers } from "@/onboarding/steps";
import { useSession } from "@/session";
import { pathOf, planName } from "@/workspace";

/** What a field holds, as its control shows it: an empty pick its first offer. */
function answerOf(view: FieldView): string {
  const offer =
    view.offers.find((each) => each.value === view.text) ??
    (view.text ? undefined : view.offers[0]);
  return offer?.label ?? (view.text || view.blank);
}

/** Every answer read back, each to be changed, then the plan named and made. */
export function CheckAnswers({ answering }: { answering: Answering }) {
  const { answers, steps, views, order } = answering;
  const session = useSession();
  const actions = useFileActions();
  const navigate = useNavigate();
  const made = useMemo(() => {
    try {
      return { plan: answers.create(), problem: null };
    } catch (thrown) {
      return { plan: null, problem: messageOf(thrown) };
    }
  }, [answers]);
  const [name, setName] = useState(made.plan?.name ?? "");
  /** The file Create writes, and the document open when it was pressed. */
  const [placing, setPlacing] = useState<{
    path: string;
    from: typeof session.document;
  } | null>(null);

  useEffect(() => {
    const isOpened =
      placing !== null &&
      session.path === placing.path &&
      session.document !== placing.from;
    if (!isOpened) return;
    keepAnswers(sessionStorage, null);
    void navigate({ to: "/overview" });
  }, [placing, session.path, session.document, navigate]);

  const asked = steps.filter((step) => order.includes(step.slug));
  return (
    <section className="max-w-prose space-y-8">
      <h1 className="text-2xl font-semibold tracking-tight">
        Check your answers
      </h1>
      {asked.map((step) => (
        <div key={step.slug} className="space-y-2">
          <h2 className="font-semibold">{step.title}</h2>
          <dl className="bg-card divide-y rounded-md border text-sm">
            {views
              .filter((view) => step.keys.includes(view.key))
              .map((view) => (
                <div
                  key={view.key}
                  className="grid grid-cols-[minmax(8rem,40%)_1fr_auto] items-baseline gap-3 px-4 py-2"
                >
                  <dt className="text-muted-foreground">{view.label}</dt>
                  <dd className="break-words">{answerOf(view)}</dd>
                  <dd>
                    <Link
                      to="/new/$step"
                      params={{ step: step.slug }}
                      search={{ field: view.key, isChanging: true }}
                      className="underline underline-offset-4"
                    >
                      Change<span className="sr-only"> {view.label}</span>
                    </Link>
                  </dd>
                </div>
              ))}
          </dl>
        </div>
      ))}
      {made.problem !== null ? (
        <Alert variant="destructive">
          <AlertDescription>{made.problem}</AlertDescription>
        </Alert>
      ) : (
        <form
          className="space-y-3"
          onSubmit={(event) => {
            event.preventDefault();
            if (!name.trim()) return;
            const file = planName(name);
            setPlacing({ path: pathOf(file), from: session.document });
            actions.add(file, made.plan.text);
          }}
        >
          <div className="grid gap-1.5">
            <label htmlFor="new-plan-name" className="text-sm font-medium">
              File name
            </label>
            <input
              id="new-plan-name"
              className={INPUT}
              value={name}
              required
              onChange={(event) => {
                setName(event.target.value);
              }}
            />
          </div>
          <div className="flex flex-wrap gap-2">
            <Button type="submit">Create plan</Button>
            <Button type="button" variant="ghost" onClick={answering.cancel}>
              Cancel
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}
