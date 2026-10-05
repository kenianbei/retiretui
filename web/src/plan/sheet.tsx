// biome-ignore lint/suspicious/noDeprecatedImports: only the positional overload is deprecated; the options one is called
import { useBlocker } from "@tanstack/react-router";
import type { Editor } from "@wasm/retiretui_wasm.js";
import { X } from "lucide-react";
import { Dialog } from "radix-ui";
import { useEffect, useMemo, useRef, useState } from "react";
import { MarginNote } from "@/components/margin-note";
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
import { gathered } from "@/lib/utils";
import { Field } from "@/plan/fields";
import { fieldId, landOnField } from "@/plan/search";
import { useSession } from "@/session";

/** Where a navigation is going, as a form's guard sees it. */
export interface Leaving {
  pathname: string;
  search: unknown;
}

interface FormSheetProps {
  editor: Editor;
  /** Whether going to `next` keeps the form open, so its edits need no question. */
  isStaying: (next: Leaving) => boolean;
  /** The field to scroll to, where a link named one. */
  field?: string;
  /** Why the last apply was refused. */
  refusal: string | null;
  apply: () => void;
  close: () => void;
}

/** A form over the page, applied or dropped whole: a sheet, or a phone's screen. */
export function FormSheet({
  editor,
  isStaying,
  field,
  refusal,
  apply,
  close,
}: FormSheetProps) {
  const { document } = useSession();
  const [changes, setChanges] = useState(0);
  const [focused, setFocused] = useState<string | null>(null);
  const refused = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);

  const blocker = useBlocker({
    shouldBlockFn: ({ next }) => !isStaying(next) && editor.isDirty,
    enableBeforeUnload: () => editor.isDirty,
    withResolver: true,
  });

  useEffect(() => {
    if (refusal) refused.current?.focus();
  }, [refusal]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: the editor changes in place; the count says when
  const views = useMemo(
    () => (document ? editor.view(document, focused ?? undefined) : []),
    [editor, document, focused, changes],
  );

  return (
    <>
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
              if (field === undefined || !body.current) return;
              if (landOnField(body.current, field)) event.preventDefault();
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
                  <MarginNote
                    ref={refused}
                    zone="shortfall"
                    role="alert"
                    tabIndex={-1}
                    className="text-sm outline-none"
                  >
                    <p>{refusal}</p>
                  </MarginNote>
                )}
                {gathered(views).map(({ group, items }) => {
                  const fields = items.map((view) => (
                    <Field
                      key={fieldId(view)}
                      view={view}
                      editor={editor}
                      changed={() => {
                        setChanges((count) => count + 1);
                      }}
                      focus={setFocused}
                    />
                  ));
                  return group ? (
                    <fieldset key={group} className="space-y-5">
                      <legend className="mb-3 text-sm font-semibold">
                        {group}
                      </legend>
                      {fields}
                    </fieldset>
                  ) : (
                    fields
                  );
                })}
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
      </Dialog.Root>
      <ApplyFirst
        title={editor.title}
        isAsking={blocker.status === "blocked"}
        keep={() => blocker.reset?.()}
        discard={() => {
          editor.discard();
          blocker.proceed?.();
        }}
        apply={() => {
          blocker.reset?.();
          apply();
        }}
      />
    </>
  );
}

interface ApplyFirstProps {
  title: string;
  isAsking: boolean;
  /** Stays in the form. */
  keep: () => void;
  /** Leaves, the edits dropped. */
  discard: () => void;
  apply: () => void;
}

/** Asks what becomes of a form's edits before it is left. */
function ApplyFirst({
  title,
  isAsking,
  keep,
  discard,
  apply,
}: ApplyFirstProps) {
  return (
    <AlertDialog open={isAsking}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Apply your edits first?</AlertDialogTitle>
          <AlertDialogDescription>
            {title} has edits that are not applied.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={keep}>Keep editing</AlertDialogCancel>
          <Button variant="outline" onClick={discard}>
            Discard edits
          </Button>
          <Button onClick={apply}>Apply</Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
