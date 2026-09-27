import { FileText, Upload } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useFileActions } from "@/files/actions";
import { useSession } from "@/session";
import { nameOf } from "@/workspace";

/** What stands in for every page while no document is open. */
export function Start() {
  const session = useSession();
  const actions = useFileActions();

  return (
    <section className="max-w-prose space-y-8">
      {session.error !== null && session.path !== null && (
        <Alert variant="destructive">
          <AlertTitle>{nameOf(session.path)} could not be opened</AlertTitle>
          <AlertDescription>{session.error}</AlertDescription>
        </Alert>
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
