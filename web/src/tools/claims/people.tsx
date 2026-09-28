import type {
  OfferedAction,
  PersonAction,
  PersonRow,
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
  const [asking, setAsking] = useState<OfferedAction | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const person = people[at];
  const rows: OptionRow<PersonRow>[] = people.map((each) => ({
    key: each.id,
    cells: each.cells,
    narrow: each.name,
    option: each,
  }));

  const run = (action: PersonAction) => {
    if (!person) return;
    try {
      const said = session.change((document) =>
        document.act(action, at, person.name),
      );
      if (said !== undefined) setOutcome({ said, isRefused: false });
    } catch (thrown) {
      setOutcome({ said: messageOf(thrown), isRefused: true });
    }
  };

  const act = (offered: OfferedAction) => {
    if (!person) return;
    const { action } = offered;
    if (action === "hold" || action === "let-vary")
      hold(person.id, action === "hold");
    else if (offered.question) setAsking(offered);
    else run(action);
  };

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
          {person.actions.map((offered) =>
            offered.action === "import" ? (
              <ImportStatement
                key={offered.action}
                index={at}
                name={person.name}
              />
            ) : (
              <Button
                key={offered.action}
                size="sm"
                variant="outline"
                onClick={() => {
                  act(offered);
                }}
              >
                {offered.label}
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
        open={asking !== null}
        onOpenChange={(isOpen) => {
          if (!isOpen) setAsking(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{asking?.question}</AlertDialogTitle>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button
              variant="destructive"
              onClick={() => {
                setAsking(null);
                if (asking) run(asking.action);
              }}
            >
              {asking?.answer}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
