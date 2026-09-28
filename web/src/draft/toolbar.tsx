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
import { IssueLink } from "@/draft/issue-link";
import { isHeld } from "@/lib/keys";
import { useSession } from "@/session";

/** Undo, redo and save by key, wherever nothing else holds the key. */
function useDraftKeys(actions: {
  undo: () => void;
  redo: () => void;
  save: () => void;
}) {
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || isHeld(event.target)) return;
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
  const { issues } = useSession();
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
        {issues.map((issue) => (
          <DropdownMenuItem
            key={issue.words}
            disabled={!issue.place}
            asChild={Boolean(issue.place)}
          >
            <IssueLink issue={issue} />
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Undo, redo, save, and what the gate finds wrong with the draft. */
export function DraftToolbar() {
  const { document, undo, redo, save } = useSession();
  const actions = useMemo(() => ({ undo, redo, save }), [undo, redo, save]);
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
        onClick={undo}
      >
        <Undo2 aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Redo"
        title="Redo"
        disabled={!document.canRedo}
        onClick={redo}
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
          onClick={save}
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
