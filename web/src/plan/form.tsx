import { useBlocker, useNavigate } from "@tanstack/react-router";
import type { Editor } from "@wasm/retiretui_wasm.js";
import { X } from "lucide-react";
import { Dialog } from "radix-ui";
import { useEffect, useMemo, useRef, useState } from "react";

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
import { messageOf } from "@/lib/utils";
import { Field } from "@/plan/fields";
import { fieldId, type PlanSearch } from "@/plan/search";
import { useSession } from "@/session";

interface ItemFormProps {
  slug: string;
  edit: number | "new";
  /** The field to scroll to, where a link named one. */
  field: string | undefined;
}

/** The editor over the item `edit` names, or `null` where there is none. */
function useEditor(slug: string, edit: number | "new"): Editor | null {
  const { document } = useSession();
  return useMemo(() => {
    if (!document) return null;
    if (edit === "new") return document.create(slug);
    try {
      return document.edit(slug, edit);
    } catch {
      return null;
    }
  }, [document, slug, edit]);
}

/** The item open in its form, over the page: a sheet, or a phone's screen. */
export function ItemForm({ slug, edit, field }: ItemFormProps) {
  const session = useSession();
  const navigate = useNavigate({ from: "/plan/$page" });
  const editor = useEditor(slug, edit);
  const [changes, setChanges] = useState(0);
  const [focused, setFocused] = useState<string | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const refused = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);

  const blocker = useBlocker({
    shouldBlockFn: ({ next }) => {
      const search = next.search as PlanSearch;
      const isStaying =
        next.pathname === `/plan/${slug}` && search.edit === edit;
      return !isStaying && editor?.isDirty === true;
    },
    enableBeforeUnload: () => editor?.isDirty === true,
    withResolver: true,
  });

  useEffect(() => {
    if (refusal) refused.current?.focus();
  }, [refusal]);

  const { document } = session;
  const views = useMemo(
    () =>
      editor && document ? editor.view(document, focused ?? undefined) : [],
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the editor changes in place; the count says when
    [editor, document, focused, changes],
  );

  if (!editor) return null;

  const close = () => {
    void navigate({ search: { item: edit === "new" ? undefined : edit } });
  };

  /** Stores the item, answering whether it went in. */
  const apply = (): boolean => {
    try {
      const stored = session.apply(editor);
      setRefusal(null);
      const item = stored ?? (edit === "new" ? undefined : edit);
      void navigate({ search: { item } });
      return true;
    } catch (thrown) {
      setRefusal(messageOf(thrown));
      return false;
    }
  };

  return (
    <Dialog.Root
      open
      onOpenChange={(isOpen) => {
        if (!isOpen) close();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/40" />
        <Dialog.Content
          aria-describedby={undefined}
          onOpenAutoFocus={(event) => {
            if (field === undefined) return;
            const shown = body.current?.querySelector<HTMLElement>(
              `[id^="${fieldId({ key: field, place: null })}"]`,
            );
            if (!shown) return;
            event.preventDefault();
            shown.scrollIntoView({ block: "center" });
            shown.focus();
          }}
          className="bg-background fixed inset-0 z-50 flex flex-col md:inset-y-0 md:right-0 md:left-auto md:w-[36rem] md:border-l md:shadow-xl"
        >
          <form
            className="flex min-h-0 flex-1 flex-col"
            onSubmit={(event) => {
              event.preventDefault();
              apply();
            }}
          >
            <header className="flex items-center gap-3 border-b px-4 py-3">
              <Dialog.Title className="mr-auto truncate text-lg font-semibold">
                {editor.title}
              </Dialog.Title>
              <Dialog.Close asChild>
                <Button variant="ghost" size="icon" aria-label="Close">
                  <X aria-hidden />
                </Button>
              </Dialog.Close>
            </header>
            <div
              ref={body}
              className="min-h-0 flex-1 space-y-5 overflow-y-auto px-4 py-5"
            >
              {refusal && (
                <div
                  ref={refused}
                  role="alert"
                  tabIndex={-1}
                  className="border-destructive text-destructive rounded-md border-l-4 px-3 py-2 text-sm"
                >
                  {refusal}
                </div>
              )}
              {views.map((view) => (
                <Field
                  key={fieldId(view)}
                  view={view}
                  editor={editor}
                  changed={() => {
                    setChanges((count) => count + 1);
                  }}
                  focus={setFocused}
                />
              ))}
            </div>
            <footer className="flex justify-end gap-2 border-t px-4 py-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
              <Dialog.Close asChild>
                <Button type="button" variant="outline">
                  Cancel
                </Button>
              </Dialog.Close>
              <Button type="submit">Apply</Button>
            </footer>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
      <AlertDialog open={blocker.status === "blocked"}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Apply your edits first?</AlertDialogTitle>
            <AlertDialogDescription>
              {editor.title} has edits that are not applied to the plan.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => blocker.reset?.()}>
              Keep editing
            </AlertDialogCancel>
            <Button
              variant="outline"
              onClick={() => {
                editor.discard();
                blocker.proceed?.();
              }}
            >
              Discard edits
            </Button>
            <Button
              onClick={() => {
                blocker.reset?.();
                apply();
              }}
            >
              Apply
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Dialog.Root>
  );
}
