import { useMatches, useNavigate, useSearch } from "@tanstack/react-router";
import { Dialog } from "radix-ui";
import { useMemo, useState, type KeyboardEvent } from "react";

import { useFileActions } from "@/files/actions";
import { INPUT, cn } from "@/lib/utils";
import { TABS, isGroup } from "@/nav";
import { NEW_PLAN_START } from "@/onboarding/steps";
import { BASIS_LABEL } from "@/overview/view-words";
import { useSession } from "@/session";
import { useGoTo } from "@/shell/go";
import { ranked } from "@/shell/match";
import { nameOf } from "@/workspace";
import { basisOf, type YearSearch } from "@/year/search";

/** Something the palette finds: a page, a plan to open, or an action. */
interface Command {
  title: string;
  kind: "Page" | "Plan" | "Action";
  run: () => void;
}

const LIST_ID = "palette-options";
const optionId = (at: number) => `palette-option-${String(at)}`;

/** Every page, plan and action the palette can find, as things stand. */
function useCommands(showKeys: () => void): Command[] {
  const session = useSession();
  const actions = useFileActions();
  const goTo = useGoTo();
  const navigate = useNavigate();
  const search: YearSearch = useSearch({ strict: false });
  const keepsBasis = useMatches({
    select: (matches) =>
      matches.some((match) => match.staticData.keeps?.includes("basis")),
  });
  const { document } = session;
  const pages: Command[] = document
    ? TABS.flatMap((tab) =>
        isGroup(tab)
          ? tab.pages.map((page) => ({
              title: `${tab.title} · ${page.title}`,
              kind: "Page" as const,
              run: () => {
                goTo(tab, page);
              },
            }))
          : [
              {
                title: tab.title,
                kind: "Page" as const,
                run: () => {
                  goTo(tab);
                },
              },
            ],
      )
    : [];
  const plans: Command[] = session.files
    .filter((path) => path !== session.path)
    .map((path) => ({
      title: `Open ${nameOf(path)}`,
      kind: "Plan",
      run: () => {
        session.open(path);
      },
    }));
  const other = basisOf(search) === "today" ? "nominal" : "today";
  const offered: (Command | false)[] = [
    {
      title: "New plan…",
      kind: "Action",
      run: () => {
        void navigate({ to: "/new/$step", params: { step: NEW_PLAN_START } });
      },
    },
    document?.isDirty === true &&
      !document.isReadOnly && {
        title: "Save",
        kind: "Action",
        run: session.save,
      },
    document !== null && {
      title: "Save as…",
      kind: "Action",
      run: actions.saveAs,
    },
    document?.canUndo === true && {
      title: "Undo",
      kind: "Action",
      run: session.undo,
    },
    document?.canRedo === true && {
      title: "Redo",
      kind: "Action",
      run: session.redo,
    },
    session.path !== null && {
      title: `Download ${nameOf(session.path)}`,
      kind: "Action",
      run: () => {
        actions.download();
      },
    },
    { title: "Upload a plan…", kind: "Action", run: actions.upload },
    session.files.length > 0 && {
      title: "Manage plans…",
      kind: "Action",
      run: actions.manage,
    },
    document !== null &&
      keepsBasis && {
        title: `Show ${BASIS_LABEL[other]}`,
        kind: "Action",
        run: () => {
          void navigate({
            to: ".",
            search: (prev) => ({
              ...prev,
              basis: other === "nominal" ? other : undefined,
            }),
            replace: true,
          });
        },
      },
    { title: "Keyboard shortcuts", kind: "Action", run: showKeys },
  ];
  return [
    ...pages,
    ...plans,
    ...offered.filter((each): each is Command => each !== false),
  ];
}

interface PaletteProps {
  isOpen: boolean;
  setOpen: (isOpen: boolean) => void;
  showKeys: () => void;
}

/** Finds a page, a plan or an action by typing part of its name. */
export function Palette({ isOpen, setOpen, showKeys }: PaletteProps) {
  const commands = useCommands(showKeys);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const shown = useMemo(() => ranked(commands, query), [commands, query]);
  const at = Math.min(active, shown.length - 1);

  const run = (command: Command | undefined) => {
    if (!command) return;
    setOpen(false);
    setTimeout(command.run);
  };
  const press = (event: KeyboardEvent) => {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (step !== undefined) {
      event.preventDefault();
      setActive((at + step + shown.length) % Math.max(shown.length, 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      run(shown[at]);
    }
  };

  return (
    <Dialog.Root
      open={isOpen}
      onOpenChange={(isNowOpen) => {
        setOpen(isNowOpen);
        setQuery("");
        setActive(0);
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/40" />
        <Dialog.Content
          aria-describedby={undefined}
          className="bg-background fixed top-[12vh] left-1/2 z-50 w-[min(36rem,calc(100vw-2rem))] -translate-x-1/2 overflow-hidden rounded-lg border shadow-xl"
        >
          <Dialog.Title className="sr-only">
            Find a page, a plan or an action
          </Dialog.Title>
          <div className="border-b p-2">
            <input
              role="combobox"
              aria-label="Find a page, a plan or an action"
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
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
