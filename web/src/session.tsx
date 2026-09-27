import { Document } from "@wasm/retiretui_wasm.js";
import {
  createContext,
  use,
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import { Workspace, fileAt, pathOf } from "@/workspace";

/** The document a path names, or why it would not open. */
interface Opened {
  path: string | null;
  document: Document | null;
  error: string | null;
}

/** The workspace and the one document open over it. */
export interface Session extends Opened {
  workspace: Workspace;
  files: string[];
  open: (path: string) => void;
  /** Writes `text` as the file `name`, replacing any such file, and opens it. */
  place: (name: string, text: string) => void;
}

const SessionContext = createContext<Session | null>(null);

function openAt(workspace: Workspace, path: string | null): Opened {
  if (path === null) return { path, document: null, error: null };
  try {
    const document = Document.open(path, (file) => workspace.read(file));
    return { path, document, error: null };
  } catch (thrown) {
    const error = thrown instanceof Error ? thrown.message : String(thrown);
    return { path, document: null, error };
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const workspace = useMemo(() => new Workspace(localStorage), []);
  const [files, setFiles] = useState(() => workspace.list());
  const [opened, setOpened] = useState(() =>
    openAt(workspace, workspace.lastOpen()),
  );

  const open = useCallback(
    (path: string) => {
      const next = openAt(workspace, path);
      if (next.document) workspace.remember(path);
      setOpened(next);
    },
    [workspace],
  );

  const place = useCallback(
    (name: string, text: string) => {
      const path = pathOf(name);
      workspace.write(path, text);
      setFiles(workspace.list());
      open(path);
    },
    [workspace, open],
  );

  useEffect(() => {
    const reload = (event: StorageEvent) => {
      const changed = fileAt(event.key);
      if (changed === undefined) return;
      setFiles(workspace.list());
      setOpened((now) =>
        changed !== null && now.document?.files().includes(changed) === false
          ? now
          : openAt(workspace, now.path),
      );
    };
    window.addEventListener("storage", reload);
    return () => {
      window.removeEventListener("storage", reload);
    };
  }, [workspace]);

  const session = useMemo(
    () => ({ ...opened, workspace, files, open, place }),
    [opened, workspace, files, open, place],
  );
  return <SessionContext value={session}>{children}</SessionContext>;
}

export function useSession(): Session {
  const session = use(SessionContext);
  if (!session) throw new Error("useSession outside a SessionProvider");
  return session;
}
