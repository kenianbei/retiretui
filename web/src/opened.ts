import { baseOf, Document, type OpenFailure } from "@wasm/retiretui_wasm.js";

import { messageOf } from "@/lib/utils";
import type { Workspace } from "@/workspace";

/** The document a path names, or why it would not open. */
export interface Opened {
  path: string | null;
  document: Document | null;
  /** Why it would not open, where it did not. */
  failure: OpenFailure | null;
}

/** Whether `thrown` is the document's own account of a failed open. */
function isOpenFailure(thrown: unknown): thrown is OpenFailure {
  return typeof thrown === "object" && thrown !== null && "headline" in thrown;
}

/** `path` opened from `workspace`, its scenario bases read from it too. */
export function openAt(workspace: Workspace, path: string | null): Opened {
  if (path === null) return { path, document: null, failure: null };
  try {
    const document = Document.open(path, (file) => workspace.read(file));
    return { path, document, failure: null };
  } catch (thrown) {
    const failure = isOpenFailure(thrown)
      ? thrown
      : { headline: messageOf(thrown), written: null };
    return { path, document: null, failure };
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
