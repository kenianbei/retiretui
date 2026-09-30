import { Download, FilePen, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { useComparedFollowMove } from "@/compare/use-compared";
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
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
} from "@/components/ui/dialog";
import { builtOn, type BaseAt } from "@/files/chain";
import { cn } from "@/lib/utils";
import { NameDialog } from "@/files/name-dialog";
import { baseIn } from "@/opened";
import { useSession } from "@/session";
import { nameOf, pathOf, planName } from "@/workspace";

/** Where plans are kept, what deletes them, and a way to ask the browser to keep them. */
export function KeepPlans() {
  const [isKept, setKept] = useState<boolean | null>(null);
  const storage = "storage" in navigator ? navigator.storage : undefined;
  useEffect(() => {
    void storage?.persisted().then(setKept);
  }, [storage]);
  return (
    <div className="text-muted-foreground space-y-2 text-sm">
      <p>
        Your plans are kept only in this browser. Clearing its site data deletes
        them, so download any you want a copy of.
      </p>
      {isKept === false && (
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            void storage?.persist().then(setKept);
          }}
        >
          Keep my plans in this browser
        </Button>
      )}
      {isKept === true && (
        <p>This browser keeps them until its site data is cleared.</p>
      )}
    </div>
  );
}

/** The file each workspace file is a scenario over, read once for the files there are. */
function useBases(): BaseAt {
  const { workspace, files } = useSession();
  const bases = useMemo(
    () => new Map(files.map((path) => [path, baseIn(workspace, path)])),
    [workspace, files],
  );
  return (path) => bases.get(path);
}

interface ManagePlansProps {
  isOpen: boolean;
  setOpen: (isOpen: boolean) => void;
  download: (path: string) => void;
}

/** Every plan in the workspace, each renamed, downloaded or deleted. */
export function ManagePlans({ isOpen, setOpen, download }: ManagePlansProps) {
  const session = useSession();
  const baseAt = useBases();
  const followMove = useComparedFollowMove();
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const target = name === null ? null : pathOf(planName(name));
  const isTaken =
    target !== null && target !== renaming && session.files.includes(target);

  return (
    <>
      <Dialog open={isOpen} onOpenChange={setOpen}>
        <DialogContent className="inset-0 flex flex-col md:inset-auto md:top-1/2 md:left-1/2 md:max-h-[80vh] md:w-[36rem] md:-translate-x-1/2 md:-translate-y-1/2 md:rounded-lg md:border">
          <DialogHeader title="Manage plans" className="border-b px-4 py-3" />
          <DialogDescription asChild>
            <div className="border-b px-4 py-3">
              <KeepPlans />
            </div>
          </DialogDescription>
          <TooltipProvider>
            <ul className="min-h-0 flex-1 divide-y overflow-y-auto">
              {session.files.map((path) => {
                const base = baseAt(path);
                const fileName = nameOf(path);
                const isCurrent = path === session.path;
                const isBadged = isCurrent || base !== undefined;
                const acts = [
                  {
                    verb: "Rename",
                    Icon: FilePen,
                    act: () => {
                      setRenaming(path);
                      setName(fileName);
                    },
                  },
                  {
                    verb: "Download",
                    Icon: Download,
                    act: () => {
                      download(path);
                    },
                  },
                  {
                    verb: "Delete",
                    Icon: Trash2,
                    act: () => {
                      setDeleting(path);
                    },
                  },
                ];
                return (
                  <li
                    key={path}
                    className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-2 gap-y-1 px-4 py-2"
                  >
                    <span title={fileName} className="truncate font-medium">
                      {fileName}
                    </span>
                    <div className={cn("flex gap-1", isBadged && "row-span-2")}>
                      {acts.map(({ verb, Icon, act }) => (
                        <Tooltip key={verb}>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              aria-label={`${verb} ${fileName}`}
                              onClick={act}
                            >
                              <Icon aria-hidden />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>{verb}</TooltipContent>
                        </Tooltip>
                      ))}
                    </div>
                    {isBadged && (
                      <div className="flex min-w-0 flex-wrap gap-1">
                        {isCurrent && <Badge>Open</Badge>}
                        {base !== undefined && (
                          <Badge variant="secondary" className="max-w-full">
                            <span className="truncate">
                              Scenario of {nameOf(base)}
                            </span>
                          </Badge>
                        )}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </TooltipProvider>
        </DialogContent>
      </Dialog>
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
          if (target !== renaming) {
            session.rename(renaming, target);
            followMove(renaming, target);
          }
          setName(null);
          setRenaming(null);
        }}
      />
      <DeleteQuestion
        path={deleting}
        baseAt={baseAt}
        close={() => {
          setDeleting(null);
        }}
        deleted={(path) => {
          followMove(path, null);
        }}
      />
    </>
  );
}

interface DeleteQuestionProps {
  path: string | null;
  baseAt: BaseAt;
  close: () => void;
  deleted: (path: string) => void;
}

/** Asks before a plan is deleted, saying what goes with it. */
function DeleteQuestion({ path, baseAt, close, deleted }: DeleteQuestionProps) {
  const session = useSession();
  const name = nameOf(path ?? "");
  const breaks = path === null ? [] : builtOn(path, session.files, baseAt);
  const hasEdits = path === session.path && session.document?.isDirty === true;
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
              if (path !== null) {
                session.removeFile(path);
                deleted(path);
              }
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
