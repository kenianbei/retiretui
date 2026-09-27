import { useRef, useState, type ChangeEvent } from "react";

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
import { useSession } from "@/session";
import { nameOf, pathOf } from "@/workspace";

interface Incoming {
  name: string;
  text: string;
}

/** Adding, uploading and downloading files, asking before one replaces another. */
export function useFileActions() {
  const session = useSession();
  const [replacing, setReplacing] = useState<Incoming | null>(null);
  const picker = useRef<HTMLInputElement>(null);

  const add = (name: string, text: string) => {
    if (session.workspace.has(pathOf(name))) setReplacing({ name, text });
    else session.place(name, text);
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

  const elements = (
    <>
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
                if (replacing) session.place(replacing.name, replacing.text);
              }}
            >
              Replace it
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );

  return {
    add,
    upload: () => picker.current?.click(),
    download,
    elements,
  };
}
