import type { Example } from "@wasm/retiretui_wasm.js";
import { examples } from "@wasm/retiretui_wasm.js";
import { createContext, use, useMemo, useState, type ReactNode } from "react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { INPUT } from "@/lib/utils";
import { useFilePicker } from "@/files/picker";
import { useSession } from "@/session";
import { nameOf, pathOf, planName } from "@/workspace";

/** A file about to be written over another of the same name. */
interface Replacing {
  name: string;
  replace: () => void;
}

/** What can be done with files, wherever a page offers it. */
interface FileActions {
  examples: Example[];
  /** Writes and opens a plan, asking first where it replaces one; then `onPlaced`. */
  add: (name: string, text: string, onPlaced?: () => void) => void;
  /** Writes a file without opening it, asking first where it replaces one; then `onWritten`. */
  write: (name: string, text: string, onWritten: () => void) => void;
  upload: () => void;
  download: () => void;
  /** Asks for a name, then writes the draft as a plan of that name. */
  saveAs: () => void;
}

const FileActionsContext = createContext<FileActions | null>(null);

/**
 * Adding, uploading and downloading files, asking before one replaces
 * another; the picker and the question are held once for every page.
 */
export function FileActionsProvider({ children }: { children: ReactNode }) {
  const session = useSession();
  const [replacing, setReplacing] = useState<Replacing | null>(null);
  const [naming, setNaming] = useState<string | null>(null);

  /** Runs `replace`, asking first where it writes over a file of `name`. */
  const writing = (name: string, replace: () => void) => {
    if (session.workspace.has(pathOf(name))) setReplacing({ name, replace });
    else replace();
  };

  const add = (name: string, text: string, onPlaced?: () => void) => {
    writing(name, () => {
      session.place(name, text, onPlaced);
    });
  };

  const saveUnder = (name: string) => {
    writing(name, () => {
      session.saveAs(name);
    });
  };

  const picker = useFilePicker(".toml", "Plan to upload", async (file) => {
    add(file.name, await file.text());
  });

  const download = () => {
    if (session.path === null) return;
    const text = session.workspace.read(session.path);
    const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
    const link = window.document.createElement("a");
    link.href = url;
    link.download = nameOf(session.path);
    link.click();
    URL.revokeObjectURL(url);
  };

  const plans = useMemo(() => examples(), []);
  const actions = {
    examples: plans,
    add,
    write: (name: string, text: string, onWritten: () => void) => {
      writing(name, () => {
        session.write(name, text);
        onWritten();
      });
    },
    upload: picker.open,
    download,
    saveAs: () => {
      setNaming(session.path === null ? "" : nameOf(session.path));
    },
  };

  return (
    <FileActionsContext value={actions}>
      {children}
      {picker.element}
      <AlertDialog
        open={replacing !== null}
        onOpenChange={(isOpen) => {
          if (!isOpen) setReplacing(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Replace {replacing?.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              Your workspace already has a plan with this name. Replacing it
              cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep the one I have</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                replacing?.replace();
              }}
            >
              Replace it
            </AlertDialogAction>
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
              if (naming === null || naming.trim() === "") return;
              setNaming(null);
              saveUnder(planName(naming));
            }}
          >
            <AlertDialogHeader>
              <AlertDialogTitle>Save as</AlertDialogTitle>
              <AlertDialogDescription>
                The plan is written to a file of this name in your workspace,
                and stays open as that file.
              </AlertDialogDescription>
            </AlertDialogHeader>
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
                Save
              </Button>
            </AlertDialogFooter>
          </form>
        </AlertDialogContent>
      </AlertDialog>
    </FileActionsContext>
  );
}

export function useFileActions(): FileActions {
  const actions = use(FileActionsContext);
  if (!actions) throw new Error("useFileActions outside a FileActionsProvider");
  return actions;
}
