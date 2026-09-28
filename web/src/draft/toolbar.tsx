import { Link } from "@tanstack/react-router";
import { issueCount } from "@wasm/retiretui_wasm.js";
import { Redo2, Save, TriangleAlert, Undo2 } from "lucide-react";
import { useEffect, useMemo } from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { placeSearch } from "@/plan/search";
import { messageOf, useSession } from "@/session";

/** Whether a key pressed at `target` is typing rather than a command. */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  );
}

/** Undo, redo and save by key, wherever nothing is being typed into. */
function useDraftKeys(actions: {
  undo: () => void;
  redo: () => void;
  save: () => void;
}) {
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || isTyping(event.target)) return;
      const key = event.key.toLowerCase();
      const action =
        key === "z"
          ? event.shiftKey
            ? actions.redo
            : actions.undo
          : key === "s"
            ? actions.save
            : undefined;
      if (!action) return;
      event.preventDefault();
      action();
    };
    window.addEventListener("keydown", press);
    return () => {
      window.removeEventListener("keydown", press);
    };
  }, [actions]);
}

function Issues() {
  const { document, revision } = useSession();
  const issues = useMemo(
    () => document?.issues() ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the document changes in place
    [document, revision],
  );
  if (issues.length === 0) return null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="text-destructive">
          <TriangleAlert aria-hidden />
          {issueCount(issues.length)}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-w-96">
        {issues.map((issue) => {
          if (!issue.place) {
            return (
              <DropdownMenuItem key={issue.words} disabled>
                {issue.words}
              </DropdownMenuItem>
            );
          }
          const { page, search } = placeSearch(issue.place);
          return (
            <DropdownMenuItem key={issue.words} asChild>
              <Link to="/plan/$page" params={{ page }} search={search}>
                {issue.words}
              </Link>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Undo, redo, save, and what the gate finds wrong with the draft. */
export function DraftToolbar() {
  const session = useSession();
  const { document, report } = session;
  const actions = useMemo(() => {
    const reported = (action: () => void) => () => {
      try {
        action();
        report(null);
      } catch (thrown) {
        report(messageOf(thrown));
      }
    };
    return {
      undo: reported(session.undo),
      redo: reported(session.redo),
      save: reported(session.save),
    };
  }, [session.undo, session.redo, session.save, report]);
  useDraftKeys(actions);
  if (!document) return null;
  const isReadOnly = document.isReadOnly;

  return (
    <div className="flex items-center gap-1">
      <Issues />
      <Button
        variant="ghost"
        size="icon"
        aria-label="Undo"
        title="Undo"
        disabled={!document.canUndo}
        onClick={actions.undo}
      >
        <Undo2 aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Redo"
        title="Redo"
        disabled={!document.canRedo}
        onClick={actions.redo}
      >
        <Redo2 aria-hidden />
      </Button>
      <span
        title={
          isReadOnly
            ? "A scenario cannot be saved over; save it as a plan of its own"
            : "Save"
        }
      >
        <Button
          variant={document.isDirty ? "default" : "outline"}
          size="sm"
          disabled={isReadOnly || !document.isDirty}
          onClick={actions.save}
        >
          <Save aria-hidden />
          Save
          {document.isDirty && (
            <span className="sr-only">, the plan has unsaved edits</span>
          )}
        </Button>
      </span>
    </div>
  );
}
