import {
  Document,
  rebased,
  type Editor,
  type PlacedIssue,
} from "@wasm/retiretui_wasm.js";
import {
  createContext,
  use,
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import { scenariosOver } from "@/files/chain";
import { messageOf } from "@/lib/utils";
import { baseIn, openAt, type Opened } from "@/opened";
import { Workspace, fileAt, nameOf, pathOf, renameAt } from "@/workspace";

/** The open document, and how many changes it has seen. */
export interface Reading {
  document: Document | null;
  revision: number;
}

/** What becomes of a draft's unsaved edits before something replaces it. */
export type Unsaved = "cancel" | "discard" | "save";

/** The workspace and the one document open over it, and its draft's edits. */
export interface Session extends Opened {
  workspace: Workspace;
  files: string[];
  /** How many times the workspace's files have been written, here or in another tab. */
  stored: number;
  /**
   * The document as of its latest change: a value that is new whenever the
   * document, which changes in place, does - what reads of it are keyed on.
   */
  reading: Reading;
  /** What the gate finds wrong with the draft. */
  issues: PlacedIssue[];
  open: (path: string) => void;
  /**
   * Writes `text` as the file `name`, replacing any such file, and opens it,
   * then runs `onPlaced`; neither where the draft's edits are kept instead.
   */
  place: (name: string, text: string, onPlaced?: () => void) => void;
  /** Writes `text` as the file `name`, replacing any such file, without opening it. */
  write: (name: string, text: string) => void;
  /** Deletes `path`, closing the document where it was the document's own file. */
  removeFile: (path: string) => void;
  /**
   * Moves `from` to `to` - which no file is - and every scenario over it
   * to name it there, the document following with its draft; a refusal is
   * reported.
   */
  rename: (from: string, to: string) => void;
  /** Stores the editor's item, answering where it now sits; throws the refusal. */
  apply: (editor: Editor) => number | undefined;
  /**
   * Runs `act` on the open document - an edit, a step of history, or what is
   * held beside the draft - and shows its outcome, answering what `act`
   * answers; throws the refusal.
   */
  change: <T>(act: (document: Document) => T) => T | undefined;
  /** Removes an item still known as `known`; a refusal is reported. */
  remove: (slug: string, index: number, known: string) => void;
  undo: () => void;
  redo: () => void;
  /** Writes the draft back to its file; a refusal is reported. */
  save: () => void;
  /** Writes the draft as the file `name`, which it then is; a refusal is reported. */
  saveAs: (name: string) => void;
  /** Whether the document's files changed in another tab under unsaved edits. */
  isChangedElsewhere: boolean;
  /** Opens the document again from its file, dropping the draft. */
  reload: () => void;
  /** Whether a replacement waits on what becomes of the draft's edits. */
  isAsking: boolean;
  answer: (choice: Unsaved) => void;
  /** What the last refused action said, until it is dismissed. */
  problem: string | null;
  report: (problem: string | null) => void;
}

const SessionContext = createContext<Session | null>(null);

export function SessionProvider({ children }: { children: ReactNode }) {
  const workspace = useMemo(() => new Workspace(localStorage), []);
  const [files, setFiles] = useState(() => workspace.list());
  const [stored, setStored] = useState(0);
  /** Lists the files again after any write, which may have added one. */
  const listed = useCallback(() => {
    setFiles(workspace.list());
    setStored((now) => now + 1);
  }, [workspace]);
  const [opened, setOpened] = useState(() =>
    openAt(workspace, workspace.lastOpen()),
  );
  const [revision, setRevision] = useState(0);
  const [isChangedElsewhere, setChangedElsewhere] = useState(false);
  const [waiting, setWaiting] = useState<(() => void) | null>(null);
  const [problem, report] = useState<string | null>(null);
  const { document, path } = opened;

  const changed = useCallback(() => {
    setRevision((now) => now + 1);
  }, []);

  const openNow = useCallback(
    (path: string) => {
      const next = openAt(workspace, path);
      if (next.document) workspace.remember(path);
      setOpened(next);
      setChangedElsewhere(false);
    },
    [workspace],
  );

  /** Runs `replace` now, or once the user says what becomes of unsaved edits. */
  const guarded = useCallback(
    (replace: () => void) => {
      if (document?.isDirty) setWaiting(() => replace);
      else replace();
    },
    [document],
  );

  const open = useCallback(
    (path: string) => {
      guarded(() => {
        openNow(path);
      });
    },
    [guarded, openNow],
  );

  const place = useCallback(
    (name: string, text: string, onPlaced?: () => void) => {
      guarded(() => {
        const path = pathOf(name);
        workspace.write(path, text);
        listed();
        openNow(path);
        onPlaced?.();
      });
    },
    [workspace, guarded, openNow, listed],
  );

  const write = useCallback(
    (name: string, text: string) => {
      workspace.write(pathOf(name), text);
      listed();
    },
    [workspace, listed],
  );

  /** Takes `to` wherever `from` was among the document's files, keeping its draft. */
  const followed = useCallback(
    (from: string, to: string) => {
      if (!document?.files().includes(from)) return;
      document.relocate(from, to);
      if (path === from) setOpened({ path: to, document, failure: null });
      changed();
    },
    [document, path, changed],
  );

  const removeFile = useCallback(
    (removed: string) => {
      workspace.remove(removed);
      listed();
      if (!document?.files().includes(removed)) return;
      setOpened(openAt(workspace, removed === path ? null : path));
    },
    [workspace, document, path, listed],
  );

  const rename = useCallback(
    (from: string, to: string) => {
      try {
        const base = (each: string) => baseIn(workspace, each);
        const over = scenariosOver(from, workspace.list(), base);
        workspace.rename(from, to);
        for (const scenario of over) {
          workspace.write(
            scenario,
            rebased(workspace.read(scenario), nameOf(to)),
          );
        }
        listed();
        followed(from, to);
      } catch (thrown) {
        report(messageOf(thrown));
      }
    },
    [workspace, listed, followed],
  );

  /** Runs `action`, reporting what it throws and clearing what was reported. */
  const attempt = useCallback(
    <Args extends unknown[]>(action: (...args: Args) => void) =>
      (...args: Args) => {
        try {
          action(...args);
          report(null);
        } catch (thrown) {
          report(messageOf(thrown));
        }
      },
    [],
  );

  const saveNow = useCallback(() => {
    if (!document || path === null) return;
    document.save((text) => {
      workspace.write(path, text);
    });
    listed();
    changed();
  }, [document, path, workspace, changed, listed]);

  const saveAs = useCallback(
    (name: string) => {
      if (!document) return;
      const target = pathOf(name);
      document.saveAs(target, (text) => {
        workspace.write(target, text);
      });
      workspace.remember(target);
      listed();
      setOpened({ path: target, document, failure: null });
      setChangedElsewhere(false);
      changed();
    },
    [document, workspace, changed, listed],
  );

  const answer = useCallback(
    (choice: Unsaved) => {
      const replace = waiting;
      setWaiting(null);
      if (choice === "cancel" || !replace) return;
      try {
        if (choice === "save") saveNow();
        replace();
      } catch (thrown) {
        report(messageOf(thrown));
      }
    },
    [waiting, saveNow],
  );

  const edits = useMemo(() => {
    const stepped =
      <Args extends unknown[]>(step: (...args: Args) => unknown) =>
      (...args: Args) => {
        step(...args);
        changed();
      };
    return {
      apply: (editor: Editor) => {
        if (!document) return undefined;
        const index = document.apply(editor);
        changed();
        return index;
      },
      change: <T,>(act: (document: Document) => T) => {
        if (!document) return undefined;
        const answer = act(document);
        changed();
        return answer;
      },
      remove: attempt(
        stepped((slug: string, index: number, known: string) => {
          document?.remove(slug, index, known);
        }),
      ),
      undo: attempt(stepped(() => document?.undo())),
      redo: attempt(stepped(() => document?.redo())),
      save: attempt(saveNow),
      saveAs: attempt(saveAs),
    };
  }, [document, changed, attempt, saveNow, saveAs]);

  const reading = useMemo(() => ({ document, revision }), [document, revision]);
  const issues = useMemo(() => reading.document?.issues() ?? [], [reading]);

  useEffect(() => {
    const follow = (event: StorageEvent) => {
      const renamed = renameAt(event.key, event.newValue);
      if (renamed) {
        followed(renamed.from, renamed.to);
        return;
      }
      const file = fileAt(event.key);
      if (file === undefined) return;
      listed();
      const isOurs = file === null || document?.files().includes(file) === true;
      if (!isOurs) return;
      if (document?.isDirty) setChangedElsewhere(true);
      else setOpened((now) => openAt(workspace, now.path));
    };
    window.addEventListener("storage", follow);
    return () => {
      window.removeEventListener("storage", follow);
    };
  }, [workspace, document, listed, followed]);

  const isDirty = document?.isDirty === true;
  useEffect(() => {
    if (!isDirty) return;
    const ask = (event: BeforeUnloadEvent) => {
      event.preventDefault();
    };
    window.addEventListener("beforeunload", ask);
    return () => {
      window.removeEventListener("beforeunload", ask);
    };
  }, [isDirty]);

  const session = useMemo(
    () => ({
      ...opened,
      workspace,
      files,
      stored,
      reading,
      issues,
      open,
      place,
      write,
      removeFile,
      rename,
      ...edits,
      isChangedElsewhere,
      reload: () => {
        if (path !== null) openNow(path);
      },
      isAsking: waiting !== null,
      answer,
      problem,
      report,
    }),
    [
      opened,
      workspace,
      files,
      stored,
      reading,
      issues,
      open,
      place,
      write,
      removeFile,
      rename,
      edits,
      isChangedElsewhere,
      path,
      openNow,
      waiting,
      answer,
      problem,
    ],
  );
  return <SessionContext value={session}>{children}</SessionContext>;
}

export function useSession(): Session {
  const session = use(SessionContext);
  if (!session) throw new Error("useSession outside a SessionProvider");
  return session;
}
