import { useNavigate, useSearch } from "@tanstack/react-router";
import type { Document } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useRef } from "react";

import { swapped, withIn, type WithSearch } from "@/compare/search";
import { releaseLane } from "@/searches";
import { openAt } from "@/opened";
import { useSession } from "@/session";

/** A compared file, or why it would not open. */
export interface ComparedFile {
  path: string;
  document: Document | null;
  error: string | null;
}

/**
 * Each of `paths` opened from the workspace, and opened again whenever any
 * file is written - here, or in another tab - since a scenario reads its
 * base from a file of its own.
 */
export function useCompared(paths: readonly string[]): ComparedFile[] {
  // ponytail: every compared file re-opens on any write; follow each one's files() if that gets slow.
  const { workspace, stored } = useSession();
  const joined = paths.join("\n");
  return useMemo(
    () =>
      joined === ""
        ? []
        : joined.split("\n").map((path) => {
            const { document, error } = openAt(workspace, path);
            return { path, document, error };
          }),
    // `stored` is what the files' writes are counted by.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [workspace, joined, stored],
  );
}

/** The worker lane a compared file's search runs in. */
export function laneOf(path: string): string {
  return `monteCarlo:${path}`;
}

/**
 * Releases each lane once it is no longer among `lanes`, and every one
 * still there when the page that searches them leaves.
 */
export function useReleased(lanes: readonly string[]) {
  const held = useRef<readonly string[]>([]);
  useEffect(() => {
    for (const lane of held.current) {
      if (!lanes.includes(lane)) releaseLane(lane);
    }
    held.current = lanes;
  }, [lanes]);
  useEffect(
    () => () => {
      held.current.forEach(releaseLane);
    },
    [],
  );
}

/**
 * Keeps the compared files in step with the document: a compared file
 * opened in its place leaves them, the document it replaces joining them
 * where it was, the baseline and the highlight following their rows; any
 * other file opened leaves nothing compared with it.
 */
export function useComparedFollowDocument() {
  const { path, document } = useSession();
  const navigate = useNavigate();
  const { with: compared }: WithSearch = useSearch({ strict: false });
  const shown = useRef({ path, document });
  useEffect(() => {
    const left = shown.current;
    shown.current = { path, document };
    if (
      left.document === document ||
      left.path === path ||
      left.path === null
    ) {
      return;
    }
    if (!compared?.length) return;
    const opened = path;
    const isSwap = opened !== null && compared.includes(opened);
    void navigate({
      to: ".",
      search: (prev) => ({
        ...prev,
        with: isSwap ? withIn(swapped(compared, opened, left.path)) : undefined,
        baseline:
          isSwap && prev.baseline !== opened
            ? (prev.baseline ?? left.path ?? undefined)
            : undefined,
        plan: undefined,
      }),
      replace: true,
    });
  }, [path, document, compared, navigate]);
}
