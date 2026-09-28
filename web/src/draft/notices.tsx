import { X } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
import { useSession } from "@/session";

/** What becomes of the draft's unsaved edits before another plan replaces it. */
export function UnsavedQuestion() {
  const session = useSession();
  const canSave = session.document?.isReadOnly === false;
  return (
    <AlertDialog
      open={session.isAsking}
      onOpenChange={(isOpen) => {
        if (!isOpen) session.answer("cancel");
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Save your edits first?</AlertDialogTitle>
          <AlertDialogDescription>
            This plan has edits that are not saved. Opening another plan drops
            them unless you save.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="outline"
            onClick={() => {
              session.answer("discard");
            }}
          >
            Discard edits
          </Button>
          {canSave && (
            <Button
              onClick={() => {
                session.answer("save");
              }}
            >
              Save
            </Button>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** A refusal to report, and the document's file changed under the draft. */
export function DraftNotices() {
  const session = useSession();
  if (!session.problem && !session.isChangedElsewhere) return null;
  return (
    <div className="mb-4 space-y-3">
      {session.isChangedElsewhere && (
        <Alert>
          <AlertTitle>The plan changed in another tab</AlertTitle>
          <AlertDescription className="flex flex-wrap items-center gap-3">
            Your edits here are not saved. Reload to see the other tab's
            version, dropping your edits, or save to keep yours.
            <Button size="sm" variant="outline" onClick={session.reload}>
              Reload
            </Button>
          </AlertDescription>
        </Alert>
      )}
      {session.problem && (
        <Alert variant="destructive" role="alert">
          <AlertTitle>That could not be done</AlertTitle>
          <AlertDescription>{session.problem}</AlertDescription>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Dismiss"
            className="absolute top-2 right-2"
            onClick={() => {
              session.report(null);
            }}
          >
            <X aria-hidden />
          </Button>
        </Alert>
      )}
    </div>
  );
}
