import { Link } from "@tanstack/react-router";
import { FilePen, FileText, Upload } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useFileActions } from "@/files/actions";
import { KeepPlans } from "@/files/manage";
import { Unopened } from "@/files/unopened";
import { NEW_PLAN_START } from "@/onboarding/steps";
import { useSession } from "@/session";
import { nameOf } from "@/workspace";

/** What stands in for every page while no document is open. */
export function Start() {
  const session = useSession();
  const actions = useFileActions();

  return (
    <section className="max-w-prose space-y-8">
      {session.failure !== null && session.path !== null && (
        <Unopened path={session.path} failure={session.failure} />
      )}
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight">
          Start with a plan
        </h1>
        <p className="text-muted-foreground">
          Plans are kept in this browser, and never leave it unless you download
          them.
        </p>
      </div>

      <div className="space-y-3">
        <h2 className="font-semibold">Make your own</h2>
        <p className="text-muted-foreground text-sm">
          A few questions about your household make a first plan to fill in.
        </p>
        <Button asChild>
          <Link to="/new/$step" params={{ step: NEW_PLAN_START }}>
            <FilePen aria-hidden />
            Answer a few questions
          </Link>
        </Button>
      </div>

      {session.files.length > 0 && (
        <div className="space-y-3">
          <h2 className="font-semibold">Your plans</h2>
          <ul className="space-y-2">
            {session.files.map((path) => (
              <li key={path}>
                <Button
                  variant="outline"
                  className="w-full justify-start"
                  onClick={() => {
                    session.open(path);
                  }}
                >
                  <FileText aria-hidden />
                  {nameOf(path)}
                </Button>
              </li>
            ))}
          </ul>
          <KeepPlans />
        </div>
      )}

      <div className="space-y-3">
        <h2 className="font-semibold">Begin from an example</h2>
        <ul className="space-y-2">
          {actions.examples.map((example) => (
            <li key={example.file}>
              <Button
                variant="outline"
                className="h-auto w-full justify-start py-3 text-left whitespace-normal"
                onClick={() => {
                  actions.add(example.file, example.text);
                }}
              >
                {example.words}
              </Button>
            </li>
          ))}
        </ul>
      </div>

      <div className="space-y-3">
        <h2 className="font-semibold">Or bring your own</h2>
        <Button onClick={actions.upload}>
          <Upload aria-hidden />
          Upload a plan
        </Button>
      </div>
    </section>
  );
}
