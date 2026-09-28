import {
  clearQuestion,
  removeQuestion,
  type Document,
  type PersonAction,
  type PersonRow,
} from "@wasm/retiretui_wasm.js";
import { useState } from "react";

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { cn, messageOf } from "@/lib/utils";
import { ImportStatement } from "@/plan/import-statement";
import { useSession } from "@/session";
import { Options, type OptionRow } from "@/tools/options";

/** Which of the columns a phone's row shows beside the name: the FRA estimate. */
const AT_FRA = 4;

/** An edit a button makes at once, as one step of history. */
const EDITS: Partial<
  Record<PersonAction, (document: Document, at: number, name: string) => string>
> = {
  "fill-career": (document, at, name) => document.fillCareer(at, name),
  "compute-benefit": (document, at, name) => document.computeBenefit(at, name),
};

/** An edit asked about first: the question, its answer, and the edit. */
const ASKED: Partial<
  Record<
    PersonAction,
    {
      question: (name: string) => string;
      answer: string;
      edit: (document: Document, at: number, name: string) => string;
    }
  >
> = {
  "clear-record": {
    question: clearQuestion,
    answer: "Clear",
    edit: (document, at, name) => document.clearRecord(at, name),
  },
  "remove-benefit": {
    question: removeQuestion,
    answer: "Remove",
    edit: (document, at, name) => document.removeBenefit(at, name),
  },
};

/** What the last action did, or why it did nothing. */
interface Outcome {
  said: string;
  isRefused: boolean;
}

/**
 * The household's People table - each person's record, income and
 * estimated benefit - over what can be done for the highlighted one.
 */
export function People({
  columns,
  people,
  at,
  highlight,
  hold,
}: {
  columns: readonly string[];
  people: PersonRow[];
  /** The highlighted person's place in the household. */
  at: number;
  highlight: (at: number) => void;
  /** Holds or lets go of the claim of the person with `id`. */
  hold: (id: string, isHeld: boolean) => void;
}) {
  const session = useSession();
  const [asking, setAsking] = useState<PersonAction | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const person = people[at];
  const rows: OptionRow<PersonRow>[] = people.map((each) => ({
    key: each.id,
    cells: each.cells,
    narrow: each.name,
    option: each,
  }));

  const run = (
    edit: (document: Document, at: number, name: string) => string,
  ) => {
    if (!person) return;
    try {
      const said = session.change((document) =>
        edit(document, at, person.name),
      );
      if (said !== undefined) setOutcome({ said, isRefused: false });
    } catch (thrown) {
      setOutcome({ said: messageOf(thrown), isRefused: true });
    }
  };

  const act = (action: PersonAction) => {
    if (!person) return;
    const edit = EDITS[action];
    if (edit) run(edit);
    else if (action === "hold" || action === "let-vary")
      hold(person.id, action === "hold");
    else setAsking(action);
  };

  const asked = asking && ASKED[asking];
  return (
    <div className="space-y-3">
      <Options
        label="People"
        columns={columns}
        rows={rows}
        narrowFigure={AT_FRA}
        highlighted={person}
        highlight={(chosen) => {
          setOutcome(null);
          highlight(people.indexOf(chosen));
        }}
      />
      {person && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium">{person.name}:</span>
          {person.actions.map(({ action, label }) =>
            action === "import" ? (
              <ImportStatement key={action} index={at} name={person.name} />
            ) : (
              <Button
                key={action}
                size="sm"
                variant="outline"
                onClick={() => {
                  act(action);
                }}
              >
                {label}
              </Button>
            ),
          )}
        </div>
      )}
      <div aria-live="polite">
        {outcome && (
          <p
            role={outcome.isRefused ? "alert" : "status"}
            className={cn(
              "text-sm",
              outcome.isRefused ? "text-destructive" : "text-muted-foreground",
            )}
          >
            {outcome.said}
          </p>
        )}
      </div>
      <AlertDialog
        open={asked !== undefined && asked !== null}
        onOpenChange={(isOpen) => {
          if (!isOpen) setAsking(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {asked && person ? asked.question(person.name) : ""}
            </AlertDialogTitle>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button
              variant="destructive"
              onClick={() => {
                setAsking(null);
                if (asked) run(asked.edit);
              }}
            >
              {asked?.answer}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
