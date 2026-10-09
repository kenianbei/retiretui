import { Link, useNavigate } from "@tanstack/react-router";
import type { Document } from "@wasm/retiretui_wasm.js";
import { useState } from "react";

import { withIn } from "@/compare/search";
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
import { NameDialog } from "@/files/name-dialog";
import { messageOf } from "@/lib/utils";
import { useSession } from "@/session";
import { pathOf, planName } from "@/workspace";
import { IN_PLACE, keptSearch } from "@/year/search";

/** What the last action did: said, and a file to open where it wrote one. */
interface Done {
  said: string;
  written?: string;
}

/** What is taken or written, and how each is said. */
export interface Chosen {
  /** What it is, after "Take " and "The ": `this ladder`, `these claims`. */
  noun: string;
  /** What is asked before it is taken. */
  question: string;
  /** What writing it does, said where its file is named. */
  described: string;
  /** The file name its scenario is offered under. */
  offered: string;
  /** Takes it into `document`, answering what was taken. */
  take: (document: Document) => string;
  /** It as a scenario to be written at `out` over `document`. */
  scenario: (document: Document, out: string) => string;
}

/**
 * Takes the highlighted option into the draft, after asking, or writes it
 * as a scenario beside the document; what each did is said beneath.
 */
export function SearchActions({
  chosen,
  isCurrent,
}: {
  chosen: Chosen;
  /** Whether it was searched over the draft as it now stands. */
  isCurrent: boolean;
}) {
  const session = useSession();
  const actions = useFileActions();
  const [isAsking, setAsking] = useState(false);
  const [naming, setNaming] = useState<string | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const navigate = useNavigate();
  const canTake = session.document?.isReadOnly === false;
  const written = done?.written;

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
      const said = session.change(chosen.take);
      if (said) setDone({ said });
    });
  };

  const write = (typed: string) => {
    const name = planName(typed);
    const path = pathOf(name);
    attempt(() => {
      const document = session.document;
      const text = document ? chosen.scenario(document, path) : "";
      setNaming(null);
      actions.write(name, text, () => {
        setDone({ said: `Wrote ${name}, compared.`, written: path });
        void navigate({
          to: ".",
          search: (prev) => ({
            ...prev,
            with: withIn([
              ...(prev.with ?? []).filter((each) => each !== path),
              path,
            ]),
          }),
          ...IN_PLACE,
        });
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
            Take {chosen.noun}
          </Button>
        )}
        <Button
          variant="outline"
          disabled={!isCurrent}
          onClick={() => {
            setProblem(null);
            setNaming(chosen.offered);
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
            {written && (
              <Button
                size="sm"
                variant="link"
                className="h-auto p-0"
                onClick={() => {
                  session.open(written);
                }}
              >
                Open it
              </Button>
            )}
            {written && (
              <Link
                to="/compare"
                search={(kept) => keptSearch(kept, ["year", "basis", "held"])}
                className="text-primary underline-offset-4 hover:underline"
              >
                Compare
              </Link>
            )}
          </p>
        )}
      </div>
      <AlertDialog open={isAsking} onOpenChange={setAsking}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Take {chosen.noun}?</AlertDialogTitle>
            <AlertDialogDescription>{chosen.question}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button onClick={take}>Take</Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <NameDialog
        name={naming}
        setName={setNaming}
        title="Write as a scenario"
        description={chosen.described}
        verb="Write"
        problem={problem}
        named={write}
      />
    </div>
  );
}
