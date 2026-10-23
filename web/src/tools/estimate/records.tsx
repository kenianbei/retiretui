import {
  type Behind,
  type Category,
  type RecordAction,
  recordMenu,
  startingWords,
  type To,
} from "@wasm/retiretui_wasm.js";
import { Ellipsis } from "lucide-react";
import { useMemo, useState } from "react";

import { columnsFor } from "@/components/columns";
import { DataTable } from "@/components/data-table";
import { LabelledSelect } from "@/components/labelled-select";
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
import { NameDialog } from "@/files/name-dialog";

/** What the dialog's category select keeps for the bucket. */
const BUCKET = "bucket";

/** What the record menu asks for in a dialog of its own. */
export interface Asking {
  kind: "words" | "name";
  /** The record behind the highlighted row it is asked of. */
  record: number;
  /** Its payee, as the rules name it. */
  payee: string;
}

/** What the record menu does with what it is given. */
export interface RecordActs {
  /** Each category, by the value kept and its words, numbered from one. */
  categories: [Category, string][];
  /** What the bucket is called; it is numbered 0. */
  bucket: string;
  act: (record: number, action: RecordAction) => void;
  ask: (asking: Asking) => void;
  /** How far a rule on some words reaches, in the client's words. */
  reach: (words: string) => string;
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
    const isIgnored = behind.ignored;
    if (isIgnored === null) return cells;
    const menu = column.display({
      id: "menu",
      header: () => <span className="sr-only">Actions</span>,
      cell: ({ row }) => (
        <RecordMenu
          record={row.index}
          cells={row.original}
          payee={behind.payees[row.index] ?? ""}
          isIgnored={isIgnored}
          acts={acts}
        />
      ),
    });
    return [...cells, menu];
  }, [behind.columns, behind.ignored, behind.payees, acts]);
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

/** What can be done with one record, in the menu's words. */
function RecordMenu({
  record,
  cells,
  payee,
  isIgnored,
  acts,
}: {
  record: number;
  cells: string[];
  payee: string;
  isIgnored: boolean;
  acts: RecordActs;
}) {
  const [record_, is, containing, ignorePayee, name] = recordMenu(
    payee,
    isIgnored,
  );
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
          {record_}
        </DropdownMenuItem>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>{is}</DropdownMenuSubTrigger>
          <DropdownMenuSubContent>
            <DropdownMenuItem
              onSelect={() => {
                acts.act(record, { "payee-is": "bucket" });
              }}
            >
              0 {acts.bucket} (bucket)
            </DropdownMenuItem>
            {acts.categories.map(([category, label], at) => (
              <DropdownMenuItem
                key={category}
                onSelect={() => {
                  acts.act(record, { "payee-is": { category } });
                }}
              >
                {at + 1} {label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuSubContent>
        </DropdownMenuSub>
        <DropdownMenuItem
          onSelect={() => {
            acts.ask({ kind: "words", record, payee });
          }}
        >
          {containing}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            acts.act(record, "ignore-payee");
          }}
        >
          {ignorePayee}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            acts.ask({ kind: "name", record, payee });
          }}
        >
          {name}
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
}: {
  asking: Asking;
  close: () => void;
  acts: RecordActs;
}) {
  const isWords = asking.kind === "words";
  const [text, setText] = useState<string | null>(() =>
    isWords ? startingWords(asking.payee) : "",
  );
  const [category, setCategory] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  return (
    <NameDialog
      name={text}
      setName={(typed) => {
        if (typed === null) close();
        else setText(typed);
      }}
      title={isWords ? "Payees containing…" : `Name ${asking.payee}`}
      description={
        isWords
          ? "Every payee whose name contains these words is sorted into the category."
          : "Payees given one name are listed as one."
      }
      label={isWords ? "Words" : "Name"}
      verb="Apply"
      problem={problem}
      named={(typed) => {
        if (isWords && category === "") {
          setProblem("Pick a category.");
          return;
        }
        const to: To =
          category === BUCKET ? "bucket" : { category: category as Category };
        acts.act(
          asking.record,
          isWords ? { containing: [typed, to] } : { name: typed },
        );
        close();
      }}
    >
      {isWords && (
        <>
          <p role="status" className="text-muted-foreground text-sm">
            {acts.reach(text ?? "")}
          </p>
          <LabelledSelect
            label="Category"
            value={category}
            options={[
              { value: "", label: "Pick one" },
              { value: BUCKET, label: `0 ${acts.bucket} (bucket)` },
              ...acts.categories.map(([value, label], at) => ({
                value,
                label: `${String(at + 1)} ${label}`,
              })),
            ]}
            onChange={setCategory}
            className="w-full"
          />
        </>
      )}
    </NameDialog>
  );
}
