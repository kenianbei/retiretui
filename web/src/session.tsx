import {
  Document,
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

import { messageOf } from "@/lib/utils";
import { Workspace, fileAt, pathOf } from "@/workspace";

/** The document a path names, or why it would not open. */
interface Opened {
  path: string | null;
  document: Document | null;
  error: string | null;
}

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
  /**
   * The document as of its latest change: a value that is new whenever the
   * document, which changes in place, does - what reads of it are keyed on.
   */
  reading: Reading;
  /** What the gate finds wrong with the draft. */
  issues: PlacedIssue[];
  open: (path: string) => void;
  /** Writes `text` as the file `name`, replacing any such file, and opens it. */
  place: (name: string, text: string) => void;
  /** Stores the editor's item, answering where it now sits; throws the refusal. */
  apply: (editor: Editor) => number | undefined;
  /** Removes an item still called `name`; a refusal is reported. */
  remove: (slug: string, index: number, name: string) => void;
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

function openAt(workspace: Workspace, path: string | null): Opened {
  if (path === null) return { path, document: null, error: null };
  try {
    const document = Document.open(path, (file) => workspace.read(file));
    return { path, document, error: null };
  } catch (thrown) {
    return { path, document: null, error: messageOf(thrown) };
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const workspace = useMemo(() => new Workspace(localStorage), []);
  const [files, setFiles] = useState(() => workspace.list());
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
    (name: string, text: string) => {
      guarded(() => {
        const path = pathOf(name);
        workspace.write(path, text);
        setFiles(workspace.list());
        openNow(path);
      });
    },
    [workspace, guarded, openNow],
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
    changed();
  }, [document, path, workspace, changed]);

  const saveAs = useCallback(
    (name: string) => {
      if (!document) return;
      const target = pathOf(name);
      document.saveAs(target, (text) => {
        workspace.write(target, text);
      });
      workspace.remember(target);
      setFiles(workspace.list());
      setOpened({ path: target, document, error: null });
      setChangedElsewhere(false);
      changed();
    },
    [document, workspace, changed],
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
      remove: attempt(
        stepped((slug: string, index: number, name: string) => {
          document?.remove(slug, index, name);
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
      const file = fileAt(event.key);
      if (file === undefined) return;
      setFiles(workspace.list());
      const isOurs = file === null || document?.files().includes(file) === true;
      if (!isOurs) return;
      if (document?.isDirty) setChangedElsewhere(true);
      else setOpened((now) => openAt(workspace, now.path));
    };
    window.addEventListener("storage", follow);
    return () => {
      window.removeEventListener("storage", follow);
    };
  }, [workspace, document]);

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
      reading,
      issues,
      open,
      place,
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
      reading,
      issues,
      open,
      place,
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
