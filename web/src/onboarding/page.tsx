import {
  Link,
  Navigate,
  useNavigate,
  useParams,
  useSearch,
} from "@tanstack/react-router";
import {
  NewPlan,
  setupSteps,
  type FieldView,
  type SetupStep,
} from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { CheckAnswers } from "@/onboarding/check";
import {
  CHECK,
  around,
  keepAnswers,
  keptAnswers,
  stepsShown,
} from "@/onboarding/steps";
import { Field } from "@/plan/fields";
import { fieldId } from "@/plan/search";

/** The answers kept this tab, or none where they no longer read. */
function resume(): NewPlan {
  const today = new Date().getFullYear();
  try {
    return new NewPlan(keptAnswers(sessionStorage), today);
  } catch {
    return new NewPlan(null, today);
  }
}

/** What every step shares: the answers, what is on show, and the order. */
export interface Answering {
  answers: NewPlan;
  steps: SetupStep[];
  views: FieldView[];
  order: string[];
  changed: () => void;
  focus: (key: string | null) => void;
  /** Forgets the answers and leaves. */
  cancel: () => void;
}

/** The new-plan questions a step at a time, then read back to be made. */
export function NewPlanPage() {
  const { step } = useParams({ from: "/new/$step" });
  const navigate = useNavigate();
  const [answers] = useState(resume);
  const [changes, setChanges] = useState(0);
  const [focused, setFocused] = useState<string | null>(null);
  const steps = useMemo(() => setupSteps(), []);
  const views = useMemo(
    () => answers.view(focused ?? undefined),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the answers change in place; the count says when
    [answers, focused, changes],
  );
  const order = stepsShown(steps, new Set(views.map((view) => view.key)));
  const answering: Answering = {
    answers,
    steps,
    views,
    order,
    changed: () => {
      keepAnswers(sessionStorage, answers.answers);
      setChanges((count) => count + 1);
    },
    focus: setFocused,
    cancel: () => {
      keepAnswers(sessionStorage, null);
      void navigate({ to: "/overview" });
    },
  };

  if (!order.includes(step)) {
    const { after } = around(order, step);
    return <Navigate to="/new/$step" params={{ step: after ?? CHECK }} />;
  }
  if (step === CHECK) return <CheckAnswers answering={answering} />;
  const shown = steps.find((each) => each.slug === step);
  return shown ? <Step step={shown} answering={answering} /> : null;
}

function Step({ step, answering }: { step: SetupStep; answering: Answering }) {
  const { field, isChanging } = useSearch({ from: "/new/$step" });
  const navigate = useNavigate();
  const { answers, views, order } = answering;
  const { before, after } = around(order, step.slug);

  useEffect(() => {
    if (field === undefined) return;
    const shown = document.getElementById(fieldId({ key: field, place: null }));
    shown?.scrollIntoView({ block: "center" });
    shown?.focus();
  }, [field]);

  return (
    <section className="max-w-prose space-y-6">
      <div className="space-y-1">
        <p className="text-muted-foreground text-sm">
          New plan · step {order.indexOf(step.slug) + 1} of {order.length}
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">{step.title}</h1>
      </div>
      <form
        className="space-y-5"
        onSubmit={(event) => {
          event.preventDefault();
          void navigate({
            to: "/new/$step",
            params: { step: isChanging ? CHECK : (after ?? CHECK) },
          });
        }}
      >
        {views
          .filter((view) => step.keys.includes(view.key))
          .map((view) => (
            <Field
              key={fieldId(view)}
              view={view}
              editor={answers}
              changed={answering.changed}
              focus={answering.focus}
            />
          ))}
        <div className="flex flex-wrap gap-2 pt-2">
          <Button type="submit">Continue</Button>
          {before && (
            <Button variant="outline" asChild>
              <Link to="/new/$step" params={{ step: before }}>
                Back
              </Link>
            </Button>
          )}
          <Button type="button" variant="ghost" onClick={answering.cancel}>
            Cancel
          </Button>
        </div>
      </form>
    </section>
  );
}
