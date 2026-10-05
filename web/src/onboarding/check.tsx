import { Link } from "@tanstack/react-router";
import type { FieldView } from "@wasm/retiretui_wasm.js";
import { useMemo, useState } from "react";

import { MarginNote } from "@/components/margin-note";
import { Button } from "@/components/ui/button";
import { useFileActions } from "@/files/actions";
import { cn, INPUT, messageOf } from "@/lib/utils";
import type { Answering } from "@/onboarding/page";
import { planName } from "@/workspace";

/** What a field holds, as its control shows it, or what its blank stands for. */
function answerOf(view: FieldView): string {
  const offer = view.offers.find((each) => each.value === view.text);
  return offer?.label ?? (view.text || view.unstated);
}

/** Every answer read back, each to be changed, then the plan named and made. */
export function CheckAnswers({ answering }: { answering: Answering }) {
  const { answers, asked, views } = answering;
  const actions = useFileActions();
  const made = useMemo(() => {
    try {
      return { plan: answers.create(), problem: null };
    } catch (thrown) {
      return { plan: null, problem: messageOf(thrown) };
    }
  }, [answers]);
  const [name, setName] = useState(made.plan?.name ?? "");

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
                  className="grid grid-cols-[minmax(8rem,min(40%,14rem))_1fr_auto] items-baseline gap-3 px-4 py-2"
                >
                  <dt className="text-muted-foreground">{view.label}</dt>
                  <dd
                    className={cn(
                      "break-words",
                      view.text === "" && "text-muted-foreground",
                    )}
                  >
                    {answerOf(view)}
                  </dd>
                  <dd>
                    <Link
                      to="/new/$step"
                      params={{ step: step.slug }}
                      search={{ field: view.key }}
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
        <MarginNote zone="shortfall" role="alert">
          <p className="text-sm">{made.problem}</p>
        </MarginNote>
      ) : (
        <form
          className="space-y-3"
          onSubmit={(event) => {
            event.preventDefault();
            if (!name.trim()) return;
            actions.add(planName(name), made.plan.text, answering.leave);
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
            <Button type="button" variant="ghost" onClick={answering.leave}>
              Cancel
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}
