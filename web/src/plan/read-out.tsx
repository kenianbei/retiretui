import { Link } from "@tanstack/react-router";
import { statementPage } from "@wasm/retiretui_wasm.js";
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
import { aligned } from "@/components/columns";
import { cn, gathered } from "@/lib/utils";
import { ImportStatement } from "@/plan/import-statement";
import { useSession } from "@/session";

const STATEMENT_PAGE = statementPage();

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
        {name && <DeleteItem slug={slug} index={index} name={name} />}
        {name &&
          slug === STATEMENT_PAGE &&
          reading.document?.isReadOnly === false && (
            <ImportStatement key={index} index={index} name={name} />
          )}
      </div>
      <ReadRows rows={rows} />
    </section>
  );
}

/** A label beside what it holds. */
export interface ReadRowView {
  label: string;
  text: string;
  /** Whether it is a figure, aligned as one. */
  isFigure?: boolean;
  /** Whether it is what a blank field stands for rather than stated. */
  is_unstated?: boolean;
  /** The heading it is gathered under with its neighbours. */
  group?: string | null;
}

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
          className="grid grid-cols-[minmax(8rem,40%)_1fr] gap-3 px-4 py-2"
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
                session.remove(slug, index, name);
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
