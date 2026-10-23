import type { Behind, Category, RecordAction } from "@wasm/retiretui_wasm.js";
import { Ellipsis } from "lucide-react";
import { useMemo, useState } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { INPUT } from "@/lib/utils";

/** What the record menu asks for in a dialog of its own. */
export interface Asking {
  kind: "words" | "name";
  /** The record behind the highlighted row it is asked of. */
  record: number;
}

/** What the record menu does with what it is given. */
export interface RecordActs {
  /** Each category, by the value kept and its words, numbered from one. */
  categories: [Category, string][];
  act: (record: number, action: RecordAction) => void;
  ask: (asking: Asking) => void;
  /** The payee of the record, as the rules name it. */
  payee: (record: number) => string;
}

/**
 * What lies behind the highlighted row, its last column the figure, and
 * where they are records a menu on each of what can be done with it.
 */
export function BehindTable({
  behind,
  acts,
}: {
  behind: Behind;
  acts: RecordActs;
}) {
  const columns = useMemo(() => {
    const column = columnsFor<string[]>();
    const cells = behind.columns.map((header, at) =>
      column.display({
        id: String(at),
        header,
        meta: { isNumeric: at === behind.columns.length - 1 },
        cell: ({ row }) => row.original[at],
      }),
    );
    const toggle = behind.toggle;
    if (!toggle) return cells;
    const menu = column.display({
      id: "menu",
      header: () => <span className="sr-only">Actions</span>,
      cell: ({ row }) => (
        <RecordMenu
          record={row.index}
          cells={row.original}
          isIgnored={toggle === "Count"}
          acts={acts}
        />
      ),
    });
    return [...cells, menu];
  }, [behind.columns, behind.toggle, acts]);
  return (
    <div className="min-w-0 space-y-3">
      <h3 className="font-semibold">{behind.title}</h3>
      <DataTable
        label={behind.title}
        columns={columns}
        rows={behind.rows}
        rowKey={(_cells, at) => String(at)}
      />
    </div>
  );
}

/** What can be done with one record. */
function RecordMenu({
  record,
  cells,
  isIgnored,
  acts,
}: {
  record: number;
  cells: string[];
  isIgnored: boolean;
  acts: RecordActs;
}) {
  const payee = acts.payee(record);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          size="icon-sm"
          variant="ghost"
          aria-label={`Actions for ${cells[1] ?? ""} on ${cells[0] ?? ""}`}
        >
          <Ellipsis aria-hidden />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onSelect={() => {
            acts.act(record, "ignore-record");
          }}
        >
          {isIgnored ? "Count this record again" : "Ignore this record"}
        </DropdownMenuItem>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>{payee} is…</DropdownMenuSubTrigger>
          <DropdownMenuSubContent>
            {acts.categories.map(([category, label], at) => (
              <DropdownMenuItem
                key={category}
                onSelect={() => {
                  acts.act(record, { "payee-is": category });
                }}
              >
                {at + 1} {label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuSubContent>
        </DropdownMenuSub>
        <DropdownMenuItem
          onSelect={() => {
            acts.ask({ kind: "words", record });
          }}
        >
          Payees containing…
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            acts.act(record, "ignore-payee");
          }}
        >
          Ignore every {payee} record
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            acts.ask({ kind: "name", record });
          }}
        >
          Name {payee}…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * Asks for the words payees contain and the category they are for,
 * saying how far the words reach as they are typed - or for the name a
 * payee is listed under.
 */
export function AskDialog({
  asking,
  close,
  acts,
  reach,
}: {
  asking: Asking;
  close: () => void;
  acts: RecordActs;
  /** How far the words reach, in the client's words. */
  reach: (words: string) => string;
}) {
  const payee = acts.payee(asking.record);
  const isWords = asking.kind === "words";
  const [text, setText] = useState(() =>
    isWords ? payee.split(/\s+/).slice(0, 2).join(" ") : "",
  );
  const [category, setCategory] = useState<Category | "">("");
  const reached = isWords ? reach(text) : null;
  const isReady = text.trim() !== "" && (!isWords || category !== "");
  return (
    <AlertDialog
      open
      onOpenChange={(isOpen) => {
        if (!isOpen) close();
      }}
    >
      <AlertDialogContent>
        <form
          className="grid gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!isReady) return;
            const action: RecordAction =
              category !== "" && isWords
                ? { containing: [text, category] }
                : { name: text };
            acts.act(asking.record, action);
            close();
          }}
        >
          <AlertDialogHeader>
            <AlertDialogTitle>
              {isWords ? "Payees containing…" : `Name ${payee}`}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {isWords
                ? "Every payee whose name contains these words is sorted into the category."
                : "Payees given one name are listed as one."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <label className="grid gap-1.5 text-sm font-medium">
            {isWords ? "Words" : "Name"}
            <input
              autoFocus
              className={INPUT}
              value={text}
              onChange={(event) => {
                setText(event.target.value);
              }}
            />
          </label>
          {reached && (
            <p role="status" className="text-muted-foreground text-sm">
              {reached}
            </p>
          )}
          {isWords && (
            <label className="grid gap-1.5 text-sm font-medium">
              Category
              <select
                className={INPUT}
                value={category}
                onChange={(event) => {
                  setCategory(event.target.value as Category);
                }}
              >
                <option value="">Pick one</option>
                {acts.categories.map(([value, label], at) => (
                  <option key={value} value={value}>
                    {at + 1} {label}
                  </option>
                ))}
              </select>
            </label>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel type="button">Cancel</AlertDialogCancel>
            <Button type="submit" disabled={!isReady}>
              Apply
            </Button>
          </AlertDialogFooter>
        </form>
      </AlertDialogContent>
    </AlertDialog>
  );
}
