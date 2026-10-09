import { useMatches, useNavigate, useSearch } from "@tanstack/react-router";
import { type KeyboardEvent, useMemo, useState } from "react";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { useFileActions } from "@/files/actions";
import { cn, INPUT } from "@/lib/utils";
import { isGroup, type Page, TABS } from "@/nav";
import { NEW_PLAN_START } from "@/onboarding/steps";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { useGoTo } from "@/shell/go";
import { ranked } from "@/shell/match";
import { nameOf } from "@/workspace";
import { basisIn, basisOf, IN_PLACE, type YearSearch } from "@/year/search";

/** Something the palette finds: a page, a plan or an action. */
interface Command {
  title: string;
  kind: "Page" | "Plan" | "Action";
  run: () => void;
}

const LIST_ID = "palette-options";
const optionId = (at: number) => `palette-option-${String(at)}`;
const NAME = "Find a page, a plan or an action";

/** Every page, while there is a document to show them over. */
function usePages(): Command[] {
  const { document } = useSession();
  const goTo = useGoTo();
  if (!document) return [];
  return TABS.flatMap((tab) =>
    (isGroup(tab) ? tab.pages : [undefined]).map((page?: Page) => ({
      title: page ? `${tab.title} · ${page.title}` : tab.title,
      kind: "Page" as const,
      run: () => {
        goTo(tab, page);
      },
    })),
  );
}

/** What can be done as things stand: each action where it can be. */
function useActions(showKeys: () => void): Command[] {
  const session = useSession();
  const files = useFileActions();
  const navigate = useNavigate();
  const search: YearSearch = useSearch({ strict: false });
  const keepsBasis = useMatches({
    select: (matches) =>
      matches.some((match) => match.staticData.keeps?.includes("basis")),
  });
  const { document, path } = session;
  const other = basisOf(search) === "today" ? "nominal" : "today";
  const offered: [boolean, string, () => void][] = [
    [
      true,
      "New plan…",
      () => {
        void navigate({ to: "/new/$step", params: { step: NEW_PLAN_START } });
      },
    ],
    [document?.isDirty === true && !document.isReadOnly, "Save", session.save],
    [document !== null, "Save as…", files.saveAs],
    [document?.canUndo === true, "Undo", session.undo],
    [document?.canRedo === true, "Redo", session.redo],
    [
      path !== null,
      `Download ${nameOf(path ?? "")}`,
      () => {
        files.download();
      },
    ],
    [true, "Upload a plan…", files.upload],
    [session.files.length > 0, "Manage plans…", files.manage],
    [
      document !== null && keepsBasis,
      `Show ${BASIS_LABEL[other]}`,
      () => {
        void navigate({
          to: ".",
          search: (prev) => ({ ...prev, basis: basisIn(other) }),
          ...IN_PLACE,
        });
      },
    ],
    [true, "Keyboard shortcuts", showKeys],
  ];
  return offered
    .filter(([isShown]) => isShown)
    .map(([, title, run]) => ({ title, kind: "Action", run }));
}

/** Every page, plan and action the palette can find, as things stand. */
function useCommands(showKeys: () => void): Command[] {
  const session = useSession();
  const plans = session.files
    .filter((path) => path !== session.path)
    .map((path) => ({
      title: `Open ${nameOf(path)}`,
      kind: "Plan" as const,
      run: () => {
        session.open(path);
      },
    }));
  return [...usePages(), ...plans, ...useActions(showKeys)];
}

interface PaletteProps {
  isOpen: boolean;
  setOpen: (isOpen: boolean) => void;
  showKeys: () => void;
}

/** Finds a page, a plan or an action by typing part of its name. */
export function Palette({ isOpen, setOpen, showKeys }: PaletteProps) {
  return (
    <Dialog open={isOpen} onOpenChange={setOpen}>
      <DialogContent
        aria-describedby={undefined}
        className="top-[12vh] left-1/2 w-[min(36rem,calc(100vw-2rem))] -translate-x-1/2 overflow-hidden rounded-lg border"
      >
        <DialogTitle className="sr-only">{NAME}</DialogTitle>
        <Finder
          run={(command) => {
            setOpen(false);
            setTimeout(command.run);
          }}
          showKeys={showKeys}
        />
      </DialogContent>
    </Dialog>
  );
}

/** A field over what it finds, walked by the arrow keys and run by Enter. */
function Finder({
  run,
  showKeys,
}: {
  run: (command: Command) => void;
  showKeys: () => void;
}) {
  const commands = useCommands(showKeys);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const shown = useMemo(() => ranked(commands, query), [commands, query]);
  const at = Math.min(active, shown.length - 1);
  const press = (event: KeyboardEvent) => {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    const command = shown[at];
    if (step !== undefined) {
      event.preventDefault();
      setActive((at + step + shown.length) % Math.max(shown.length, 1));
    } else if (event.key === "Enter" && command) {
      event.preventDefault();
      run(command);
    }
  };

  return (
    <>
      <div className="border-b p-2">
        <input
          role="combobox"
          aria-label={NAME}
          aria-expanded
          aria-controls={LIST_ID}
          aria-autocomplete="list"
          aria-activedescendant={at >= 0 ? optionId(at) : undefined}
          value={query}
          placeholder="Type a page, a plan or an action"
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={press}
          className={cn(INPUT, "w-full")}
        />
      </div>
      <ul
        id={LIST_ID}
        role="listbox"
        aria-label="Pages, plans and actions"
        className="max-h-80 overflow-y-auto p-1"
      >
        {shown.map((command, place) => (
          <li
            key={`${command.kind}:${command.title}`}
            id={optionId(place)}
            role="option"
            aria-selected={place === at}
            onMouseMove={() => {
              setActive(place);
            }}
            onClick={() => {
              run(command);
            }}
            className={cn(
              "flex cursor-pointer items-center justify-between gap-3 rounded-md px-3 py-2 text-sm",
              place === at && "bg-accent",
            )}
          >
            <span>{command.title}</span>
            <span className="text-muted-foreground text-xs">
              {command.kind}
            </span>
          </li>
        ))}
      </ul>
      {shown.length === 0 && (
        <p className="text-muted-foreground px-4 py-3 text-sm">
          Nothing is called that.
        </p>
      )}
    </>
  );
}
