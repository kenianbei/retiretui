import { Link } from "@tanstack/react-router";
import { Pencil, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

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

/** An item's every field it has a use for, in the form's words. */
export function ReadOut({
  slug,
  index,
  name,
}: {
  slug: string;
  index: number;
  /** The item's name, where it is one of many and may be deleted. */
  name?: string;
}) {
  const { document, revision } = useSession();
  const rows = useMemo(
    () => {
      try {
        return document?.readOut(slug, index) ?? null;
      } catch {
        return null;
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the document changes in place
    [document, revision, slug, index],
  );
  if (!rows) return null;
  return (
    <section aria-label={name ?? "Details"} className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        {name && <h2 className="mr-auto text-lg font-semibold">{name}</h2>}
        <Button size="sm" asChild>
          <Link
            to="/plan/$page"
            params={{ page: slug }}
            search={(held) => ({ ...held, edit: index })}
          >
            <Pencil aria-hidden />
            Edit
          </Link>
        </Button>
        {name && <DeleteItem slug={slug} index={index} name={name} />}
      </div>
      <dl className="bg-card divide-y rounded-md border text-sm">
        {rows.map(([label, text]) => (
          <div
            key={label}
            className="grid grid-cols-[minmax(8rem,40%)_1fr] gap-3 px-4 py-2"
          >
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="break-words">{text}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

function DeleteItem({
  slug,
  index,
  name,
}: {
  slug: string;
  index: number;
  name: string;
}) {
  const session = useSession();
  const [isAsking, setAsking] = useState(false);
  return (
    <>
      <Button
        size="sm"
        variant="outline"
        disabled={session.document?.isReadOnly}
        onClick={() => {
          setAsking(true);
        }}
      >
        <Trash2 aria-hidden />
        Delete
      </Button>
      <AlertDialog open={isAsking} onOpenChange={setAsking}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete {name}?</AlertDialogTitle>
            <AlertDialogDescription>
              It leaves the plan as one edit, which Undo brings back.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep it</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                try {
                  session.remove(slug, index, name);
                  session.report(null);
                } catch (thrown) {
                  session.report(messageOf(thrown));
                }
              }}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
