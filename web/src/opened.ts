import { Document, baseOf, type OpenFailure } from "@wasm/retiretui_wasm.js";

import { messageOf } from "@/lib/utils";
import type { Workspace } from "@/workspace";

/** The document a path names, or why it would not open. */
export interface Opened {
  path: string | null;
  document: Document | null;
  /** Why it would not open, in a line. */
  error: string | null;
  /** The file that failed, and where in it, where the document says. */
  failure: OpenFailure | null;
}

/** Whether `thrown` is the document's own account of a failed open. */
function isOpenFailure(thrown: unknown): thrown is OpenFailure {
  return typeof thrown === "object" && thrown !== null && "headline" in thrown;
}

/** `path` opened from `workspace`, its scenario bases read from it too. */
export function openAt(workspace: Workspace, path: string | null): Opened {
  if (path === null)
    return { path, document: null, error: null, failure: null };
  try {
    const document = Document.open(path, (file) => workspace.read(file));
    return { path, document, error: null, failure: null };
  } catch (thrown) {
    const failure = isOpenFailure(thrown) ? thrown : null;
    const error = failure?.headline ?? messageOf(thrown);
    return { path, document: null, error, failure };
  }
}

/** The file a workspace file is a scenario over, where it is one. */
export function baseIn(workspace: Workspace, path: string): string | undefined {
  try {
    return baseOf(path, workspace.read(path));
  } catch {
    return undefined;
  }
}
