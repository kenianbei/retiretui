import type { LadderOption } from "@wasm/retiretui_wasm.js";
import { useState } from "react";

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { useFileActions } from "@/files/actions";
import { INPUT, messageOf } from "@/lib/utils";
import { useSession } from "@/session";
import { percentOf } from "@/tools/search";
import { nameOf, pathOf, planName } from "@/workspace";

/** What the last action did: said, and a file to open where it wrote one. */
interface Done {
  said: string;
  written?: string;
}

/** The file name a ladder's scenario is offered under. */
function offeredName(path: string | null, option: LadderOption): string {
  const stem = nameOf(path ?? "plan").replace(/\.toml$/, "");
  return `${stem}-ladder-${String(percentOf(option.rate))}.toml`;
}

/**
 * Takes the highlighted ladder into the draft, after asking, or writes it
 * as a scenario beside the document; what each did is said beneath.
 */
export function LadderActions({
  destination,
  option,
  isCurrent,
}: {
  destination: string;
  option: LadderOption;
  /** Whether the ladder was searched over the draft as it now stands. */
  isCurrent: boolean;
}) {
  const session = useSession();
  const actions = useFileActions();
  const [isAsking, setAsking] = useState(false);
  const [naming, setNaming] = useState<string | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const canTake = session.document?.isReadOnly === false;

  const attempt = (action: () => void) => {
    try {
      action();
      setProblem(null);
    } catch (thrown) {
      setDone(null);
      setProblem(messageOf(thrown));
    }
  };

  const take = () => {
    setAsking(false);
    attempt(() => {
      const said = session.takeLadder(destination, option.steps);
      if (said) setDone({ said });
    });
  };

  const write = (typed: string) => {
    const name = planName(typed);
    attempt(() => {
      const text =
        session.document?.ladderScenario(
          pathOf(name),
          destination,
          option.steps,
        ) ?? "";
      setNaming(null);
      actions.write(name, text, () => {
        setDone({ said: `Wrote ${name}.`, written: pathOf(name) });
      });
    });
  };

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-2">
        {canTake && (
          <Button
            disabled={!isCurrent}
            onClick={() => {
              setAsking(true);
            }}
          >
            Take this ladder
          </Button>
        )}
        <Button
          variant="outline"
          disabled={!isCurrent}
          onClick={() => {
            setProblem(null);
            setNaming(offeredName(session.path, option));
          }}
        >
          Write as a scenario
        </Button>
      </div>
      <div aria-live="polite" className="text-sm">
        {problem && (
          <p role="alert" className="text-destructive">
            {problem}
          </p>
        )}
        {done && (
          <p className="flex flex-wrap items-center gap-2">
            {done.said}
            {done.written && (
              <Button
                size="sm"
                variant="link"
                className="h-auto p-0"
                onClick={() => {
                  if (done.written) session.open(done.written);
                }}
              >
                Open it
              </Button>
            )}
          </p>
        )}
      </div>
      <AlertDialog open={isAsking} onOpenChange={setAsking}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Take this ladder?</AlertDialogTitle>
            <AlertDialogDescription>{option.question}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button onClick={take}>Take</Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <AlertDialog
        open={naming !== null}
        onOpenChange={(isOpen) => {
          if (!isOpen) setNaming(null);
        }}
      >
        <AlertDialogContent>
          <form
            className="grid gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (naming?.trim()) write(naming);
            }}
          >
            <AlertDialogHeader>
              <AlertDialogTitle>Write as a scenario</AlertDialogTitle>
              <AlertDialogDescription>
                The {option.label} ladder is written as a scenario over this
                plan, to a file of this name in your workspace.
              </AlertDialogDescription>
            </AlertDialogHeader>
            {problem && (
              <p role="alert" className="text-destructive text-sm">
                {problem}
              </p>
            )}
            <label className="grid gap-1.5 text-sm font-medium">
              File name
              <input
                autoFocus
                className={INPUT}
                value={naming ?? ""}
                onChange={(event) => {
                  setNaming(event.target.value);
                }}
              />
            </label>
            <AlertDialogFooter>
              <AlertDialogCancel type="button">Cancel</AlertDialogCancel>
              <Button type="submit" disabled={naming?.trim() === ""}>
                Write
              </Button>
            </AlertDialogFooter>
          </form>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
