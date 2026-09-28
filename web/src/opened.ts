import { Document } from "@wasm/retiretui_wasm.js";

import { messageOf } from "@/lib/utils";
import type { Workspace } from "@/workspace";

/** The document a path names, or why it would not open. */
export interface Opened {
  path: string | null;
  document: Document | null;
  error: string | null;
}

/** `path` opened from `workspace`, its scenario bases read from it too. */
export function openAt(workspace: Workspace, path: string | null): Opened {
  if (path === null) return { path, document: null, error: null };
  try {
    const document = Document.open(path, (file) => workspace.read(file));
    return { path, document, error: null };
  } catch (thrown) {
    return { path, document: null, error: messageOf(thrown) };
  }
}
