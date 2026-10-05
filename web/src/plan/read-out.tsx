import { Link } from "@tanstack/react-router";
import { type ReadRow, statementPage } from "@wasm/retiretui_wasm.js";
import { Pencil, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { aligned } from "@/components/columns";
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
import { cn, gathered } from "@/lib/utils";
import { ImportStatement } from "@/plan/import-statement";
import { useSession } from "@/session";

const STATEMENT_PAGE = statementPage();

/** An item's every field it has a use for, in the form's words. */
export function ReadOut({
  slug,
  index,
  name,
  known,
}: {
  slug: string;
  index: number;
  /** The item's name, where it is one of many and may be deleted. */
  name?: string;
  /** What the item is known by, which deleting it checks. */
  known?: string;
}) {
  const { reading } = useSession();
  const rows = useMemo(() => {
    try {
      return reading.document?.readOut(slug, index) ?? null;
    } catch {
      return null;
    }
  }, [reading, slug, index]);
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
        {name && known !== undefined && (
          <DeleteItem slug={slug} index={index} name={name} known={known} />
        )}
        {known !== undefined &&
          slug === STATEMENT_PAGE &&
          reading.document?.isReadOnly === false && (
            <ImportStatement key={index} index={index} known={known} />
          )}
      </div>
      <ReadRows rows={rows} />
    </section>
  );
}

/** A label beside what it holds, as the client reads it out, and whether
 * it is a figure, aligned as one. */
export type ReadRowView = Pick<ReadRow, "label" | "text"> &
  Partial<Pick<ReadRow, "is_unstated" | "group">> & { isFigure?: boolean };

/**
 * Each label beside what it holds, what a blank stands for muted, and
 * neighbours gathered under a heading of their own.
 */
export function ReadRows({
  rows,
  isFlush = false,
}: {
  rows: readonly ReadRowView[];
  /** Set in a container of its own: no frame but a rule above. */
  isFlush?: boolean;
}) {
  const runs = gathered(rows);
  if (runs.every((run) => run.group === null))
    return <RowList rows={rows} isFlush={isFlush} />;
  return (
    <div className="space-y-4">
      {runs.map((run) => (
        <section
          key={run.group ?? run.items[0]?.label}
          aria-label={run.group ?? undefined}
          className="space-y-1.5"
        >
          {run.group && <h3 className="text-sm font-semibold">{run.group}</h3>}
          <RowList rows={run.items} isFlush={isFlush} />
        </section>
      ))}
    </div>
  );
}

function RowList({
  rows,
  isFlush,
}: {
  rows: readonly ReadRowView[];
  isFlush: boolean;
}) {
  return (
    <dl
      className={cn(
        "bg-card divide-y rounded-md border text-sm",
        isFlush && "rounded-none border-x-0 border-b-0 bg-transparent",
      )}
    >
      {rows.map(({ label, text, isFigure, is_unstated }) => (
        <div
          key={label}
          className="grid grid-cols-[minmax(8rem,min(40%,14rem))_1fr] gap-3 px-4 py-2"
        >
          <dt className="text-muted-foreground">{label}</dt>
          <dd
            className={cn(
              "break-words",
              isFigure && aligned(true),
              is_unstated && "text-muted-foreground",
            )}
          >
            {text}
          </dd>
        </div>
      ))}
    </dl>
  );
}

function DeleteItem({
  slug,
  index,
  name,
  known,
}: {
  slug: string;
  index: number;
  name: string;
  known: string;
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
              variant="destructive"
              onClick={() => {
                session.remove(slug, index, known);
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
