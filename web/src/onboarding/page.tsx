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
  type Step as SetupStep,
} from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { CheckAnswers } from "@/onboarding/check";
import {
  CHECK,
  around,
  keepAnswers,
  keptAnswers,
  stepsAsked,
} from "@/onboarding/steps";
import { Field } from "@/plan/fields";
import { fieldId, landOnField } from "@/plan/search";

/** The answers kept this tab, or none where they no longer read. */
function resume(): NewPlan {
  const today = new Date().getFullYear();
  try {
    return new NewPlan(keptAnswers(sessionStorage), today);
  } catch {
    return new NewPlan(null, today);
  }
}

/** What every step shares: the answers, what is on show, and the steps asked. */
export interface Answering {
  answers: NewPlan;
  asked: SetupStep[];
  views: FieldView[];
  changed: () => void;
  focus: (key: string | null) => void;
  /** Forgets the answers and goes to the Overview. */
  leave: () => void;
}

/** The new-plan questions a step at a time, then read back to be made. */
export function NewPlanPage() {
  const { step } = useParams({ from: "/new/$step" });
  const navigate = useNavigate();
  const [answers] = useState(resume);
  const [changes, setChanges] = useState(0);
  const [focused, setFocused] = useState<string | null>(null);
  const steps = useMemo(() => setupSteps(), []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the answers change in place; the count says when
  const views = useMemo(
    () => answers.view(focused ?? undefined),
    [answers, focused, changes],
  );
  const asked = stepsAsked(steps, new Set(views.map((view) => view.key)));
  const answering: Answering = {
    answers,
    asked,
    views,
    changed: () => {
      keepAnswers(sessionStorage, answers.answers);
      setChanges((count) => count + 1);
    },
    focus: setFocused,
    leave: () => {
      keepAnswers(sessionStorage, null);
      void navigate({ to: "/overview" });
    },
  };

  if (step === CHECK) return <CheckAnswers answering={answering} />;
  const shown = asked.find((each) => each.slug === step);
  if (!shown) {
    return (
      <Navigate to="/new/$step" params={{ step: asked[0]?.slug ?? CHECK }} />
    );
  }
  return <Step step={shown} answering={answering} />;
}

function Step({ step, answering }: { step: SetupStep; answering: Answering }) {
  const { field } = useSearch({ from: "/new/$step" });
  const navigate = useNavigate();
  const { answers, asked, views } = answering;
  const { place, count, before, after } = around(asked, step.slug);

  useEffect(() => {
    if (field !== undefined) landOnField(document, field);
  }, [field]);

  return (
    <section className="max-w-prose space-y-6">
      <div className="space-y-1">
        <p className="text-muted-foreground text-sm">
          New plan · step {place} of {count}
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">{step.title}</h1>
      </div>
      <form
        className="space-y-5"
        onSubmit={(event) => {
          event.preventDefault();
          const next = field === undefined ? after : CHECK;
          void navigate({ to: "/new/$step", params: { step: next ?? CHECK } });
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
          <Button type="button" variant="ghost" onClick={answering.leave}>
            Cancel
          </Button>
        </div>
      </form>
    </section>
  );
}
