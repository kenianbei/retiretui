import type { Example } from "@wasm/retiretui_wasm.js";
import { examples } from "@wasm/retiretui_wasm.js";
import {
  createContext,
  use,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type ReactNode,
} from "react";

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
import { messageOf, useSession } from "@/session";
import { nameOf, pathOf } from "@/workspace";

/** A file about to be written over another of the same name. */
interface Replacing {
  name: string;
  replace: () => void;
}

const PLAN_EXTENSION = ".toml";

/** A typed name as a plan file's: `.toml` added where it is missing. */
export function planName(typed: string): string {
  const name = typed.trim();
  return name.endsWith(PLAN_EXTENSION) ? name : `${name}${PLAN_EXTENSION}`;
}

/** What can be done with files, wherever a page offers it. */
interface FileActions {
  examples: Example[];
  add: (name: string, text: string) => void;
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
  const picker = useRef<HTMLInputElement>(null);

  /** Runs `replace`, asking first where it writes over a file of `name`. */
  const writing = (name: string, replace: () => void) => {
    const path = pathOf(name);
    if (session.workspace.has(path) && path !== session.path)
      setReplacing({ name, replace });
    else replace();
  };

  const add = (name: string, text: string) => {
    writing(name, () => {
      session.place(name, text);
    });
  };

  const saveUnder = (name: string) => {
    writing(name, () => {
      try {
        session.saveAs(name);
        session.report(null);
      } catch (thrown) {
        session.report(messageOf(thrown));
      }
    });
  };

  const readPicked = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (file) add(file.name, await file.text());
  };

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
    upload: () => picker.current?.click(),
    download,
    saveAs: () => {
      setNaming(session.path === null ? "" : nameOf(session.path));
    },
  };

  return (
    <FileActionsContext value={actions}>
      {children}
      <input
        ref={picker}
        type="file"
        accept=".toml"
        hidden
        onChange={(event) => void readPicked(event)}
      />
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
                className="border-input bg-background h-9 rounded-md border px-3 text-base md:text-sm"
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
