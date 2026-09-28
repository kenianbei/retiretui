import type { ReactNode } from "react";

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
import { INPUT } from "@/lib/utils";

interface NameDialogProps {
  /** The name being typed; the dialog is open while there is one. */
  name: string | null;
  setName: (name: string | null) => void;
  title: string;
  description: ReactNode;
  /** What the button that takes the name says. */
  verb: string;
  /** Why the last name given was refused. */
  problem?: string | null;
  named: (name: string) => void;
}

/** Asks for a file name, and hands it over once one is given. */
export function NameDialog({
  name,
  setName,
  title,
  description,
  verb,
  problem,
  named,
}: NameDialogProps) {
  return (
    <AlertDialog
      open={name !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) setName(null);
      }}
    >
      <AlertDialogContent>
        <form
          className="grid gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (name?.trim()) named(name);
          }}
        >
          <AlertDialogHeader>
            <AlertDialogTitle>{title}</AlertDialogTitle>
            <AlertDialogDescription>{description}</AlertDialogDescription>
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
              value={name ?? ""}
              onChange={(event) => {
                setName(event.target.value);
              }}
            />
          </label>
          <AlertDialogFooter>
            <AlertDialogCancel type="button">Cancel</AlertDialogCancel>
            <Button type="submit" disabled={name?.trim() === ""}>
              {verb}
            </Button>
          </AlertDialogFooter>
        </form>
      </AlertDialogContent>
    </AlertDialog>
  );
}
