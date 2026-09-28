import { Download, FilePen, Trash2, X } from "lucide-react";
import { Dialog } from "radix-ui";
import { useEffect, useState } from "react";

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { basePath, builtOn } from "@/files/chain";
import { NameDialog } from "@/files/name-dialog";
import { baseIn } from "@/opened";
import { useSession } from "@/session";
import { nameOf, pathOf, planName } from "@/workspace";

/** Whether the browser keeps the page's storage until its user clears it. */
function useIsKept(): [boolean | null, () => void] {
  const [isKept, setKept] = useState<boolean | null>(null);
  const storage = "storage" in navigator ? navigator.storage : undefined;
  useEffect(() => {
    void storage?.persisted().then(setKept);
  }, [storage]);
  const ask = () => {
    void storage?.persist().then(setKept);
  };
  return [isKept, ask];
}

/** Where plans are kept, what deletes them, and a way to ask the browser to keep them. */
export function KeepPlans() {
  const [isKept, ask] = useIsKept();
  return (
    <div className="text-muted-foreground space-y-2 text-sm">
      <p>
        Your plans are kept only in this browser. Clearing its site data deletes
        them, so download any you want a copy of.
      </p>
      {isKept === false && (
        <Button variant="outline" size="sm" onClick={ask}>
          Keep my plans in this browser
        </Button>
      )}
      {isKept === true && (
        <p>This browser keeps them until its site data is cleared.</p>
      )}
    </div>
  );
}

interface ManagePlansProps {
  isOpen: boolean;
  setOpen: (isOpen: boolean) => void;
  download: (path: string) => void;
}

/** Every plan in the workspace, each renamed, downloaded or deleted. */
export function ManagePlans({ isOpen, setOpen, download }: ManagePlansProps) {
  const session = useSession();
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const baseAt = (path: string) => baseIn(session.workspace, path);
  const target = name === null ? null : pathOf(planName(name));
  const isTaken =
    target !== null && target !== renaming && session.files.includes(target);

  return (
    <>
      <Dialog.Root open={isOpen} onOpenChange={setOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-40 bg-black/40" />
          <Dialog.Content className="bg-background fixed inset-0 z-50 flex flex-col md:inset-auto md:top-1/2 md:left-1/2 md:max-h-[80vh] md:w-[36rem] md:-translate-x-1/2 md:-translate-y-1/2 md:rounded-lg md:border md:shadow-xl">
            <header className="flex items-center gap-3 border-b px-4 py-3">
              <Dialog.Title className="mr-auto text-lg font-semibold">
                Manage plans
              </Dialog.Title>
              <Dialog.Close asChild>
                <Button variant="ghost" size="icon" aria-label="Close">
                  <X aria-hidden />
                </Button>
              </Dialog.Close>
            </header>
            <Dialog.Description asChild>
              <div className="border-b px-4 py-3">
                <KeepPlans />
              </div>
            </Dialog.Description>
            <ul className="min-h-0 flex-1 divide-y overflow-y-auto">
              {session.files.map((path) => {
                const base = baseAt(path);
                const name = nameOf(path);
                return (
                  <li
                    key={path}
                    className="flex flex-wrap items-center gap-2 px-4 py-2"
                  >
                    <span className="mr-auto min-w-0 truncate font-medium">
                      {name}
                    </span>
                    {path === session.path && <Badge>Open</Badge>}
                    {base !== undefined && (
                      <Badge variant="secondary">
                        Scenario of {nameOf(basePath(base))}
                      </Badge>
                    )}
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Rename ${name}`}
                        onClick={() => {
                          setRenaming(path);
                          setName(name);
                        }}
                      >
                        <FilePen aria-hidden />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Download ${name}`}
                        onClick={() => {
                          download(path);
                        }}
                      >
                        <Download aria-hidden />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Delete ${name}`}
                        onClick={() => {
                          setDeleting(path);
                        }}
                      >
                        <Trash2 aria-hidden />
                      </Button>
                    </div>
                  </li>
                );
              })}
            </ul>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
      <NameDialog
        name={name}
        setName={setName}
        title={`Rename ${nameOf(renaming ?? "")}`}
        description="Scenarios built on it are changed to name it by its new name."
        verb="Rename"
        problem={
          isTaken
            ? `Your workspace already has a plan named ${nameOf(target)}.`
            : null
        }
        named={() => {
          if (isTaken || renaming === null || target === null) return;
          if (target !== renaming) session.rename(renaming, target);
          setName(null);
          setRenaming(null);
        }}
      />
      <DeleteQuestion
        path={deleting}
        breaks={
          deleting === null ? [] : builtOn(deleting, session.files, baseAt)
        }
        hasEdits={
          deleting === session.path && session.document?.isDirty === true
        }
        close={() => {
          setDeleting(null);
        }}
        remove={session.remove}
      />
    </>
  );
}

interface DeleteQuestionProps {
  path: string | null;
  /** The scenarios that will no longer open. */
  breaks: string[];
  /** Whether it is the document, with edits not yet saved. */
  hasEdits: boolean;
  close: () => void;
  remove: (path: string) => void;
}

/** Asks before a plan is deleted, saying what goes with it. */
function DeleteQuestion({
  path,
  breaks,
  hasEdits,
  close,
  remove,
}: DeleteQuestionProps) {
  const name = nameOf(path ?? "");
  return (
    <AlertDialog
      open={path !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) close();
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {name}?</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2">
              <p>
                It is deleted from this browser, and cannot be brought back.
              </p>
              {hasEdits && <p>Its unsaved edits are lost too.</p>}
              {breaks.length > 0 && (
                <p>
                  These scenarios are built on it, and will no longer open:{" "}
                  {breaks.map(nameOf).join(", ")}.
                </p>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Keep it</AlertDialogCancel>
          <Button
            variant="destructive"
            onClick={() => {
              if (path !== null) remove(path);
              close();
            }}
          >
            Delete
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
