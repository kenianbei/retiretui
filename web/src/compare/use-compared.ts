import { useNavigate, useSearch } from "@tanstack/react-router";
import type { Document, Searched } from "@wasm/retiretui_wasm.js";
import { useEffect, useMemo, useRef } from "react";

import { swapped, type WithSearch, withIn } from "@/compare/search";
import { openAt } from "@/opened";
import { releaseLane, useSuccesses } from "@/searches";
import { useSession } from "@/session";

/** A compared file, or why it would not open. */
export interface ComparedFile {
  path: string;
  document: Document | null;
  error: string | null;
  /** Its text to search through random markets; none while it has issues. */
  text: string;
}

/** A plan compared, and where its search through random markets has got to. */
export type Row = ComparedFile & { searched: Searched };

/** The lane the Overview searches the document in, shared with it. */
const DOCUMENT_LANE = "monteCarlo";

/** The worker lane a compared file's search runs in. */
function laneOf(path: string): string {
  return `monteCarlo:${path}`;
}

/**
 * Each of `paths` opened from the workspace, and opened again whenever any
 * file is written - here, or in another tab - since a scenario reads its
 * base from a file of its own.
 */
function useCompared(paths: readonly string[]): ComparedFile[] {
  // ponytail: every compared file re-opens on any write; follow each one's files() if that gets slow.
  const { workspace, stored } = useSession();
  const joined = paths.join("\n");
  // biome-ignore lint/correctness/useExhaustiveDependencies: `stored` is what the files' writes are counted by
  return useMemo(
    () =>
      joined === ""
        ? []
        : joined.split("\n").map((path) => {
            const { document, failure } = openAt(workspace, path);
            const error = failure?.headline ?? null;
            const text =
              document && document.issues().length === 0
                ? document.planText()
                : "";
            return { path, document, error, text };
          }),
    [workspace, joined, stored],
  );
}

/**
 * The document, then each of `compared` opened, each with where its search
 * through random markets has got to - the document's shared with the
 * Overview, each compared file's in a lane of its own.
 */
export function useRows(compared: readonly string[]): [Row, ...Row[]] {
  const { reading, issues, path, failure } = useSession();
  const opened = useCompared(compared);
  useReleased(useMemo(() => compared.map(laneOf), [compared]));
  const text = useMemo(
    () => (issues.length === 0 ? (reading.document?.planText() ?? "") : ""),
    [reading, issues],
  );
  const searched = useSuccesses(
    [
      { plan: text, lane: DOCUMENT_LANE },
      ...opened.map((plan) => ({ plan: plan.text, lane: laneOf(plan.path) })),
    ],
    true,
  );
  return useMemo(() => {
    const waiting: Searched = { kind: "waiting" };
    const document = reading.document;
    return [
      {
        path: path ?? "",
        document,
        error: failure?.headline ?? null,
        text,
        searched: searched[0] ?? waiting,
      },
      ...opened.map((plan, at) => ({
        ...plan,
        searched: searched[at + 1] ?? waiting,
      })),
    ];
  }, [reading, path, failure, text, opened, searched]);
}

/**
 * Releases each lane once it is no longer among `lanes`, and every one
 * still there when the page that searches them leaves.
 */
function useReleased(lanes: readonly string[]) {
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

/** `path` once `from` is renamed `to`, or deleted where `to` is `null`. */
function movedPath(path: string | undefined, from: string, to: string | null) {
  return path === from ? (to ?? undefined) : path;
}

/**
 * Follows a compared file renamed or deleted: under its new name, or gone,
 * in the compared files, the baseline and the highlight.
 */
export function useComparedFollowMove() {
  const navigate = useNavigate();
  const { with: compared }: WithSearch = useSearch({ strict: false });
  return (from: string, to: string | null) => {
    if (!compared?.includes(from)) return;
    const moved = (path: string) => movedPath(path, from, to) ?? [];
    void navigate({
      to: ".",
      search: (prev) => ({
        ...prev,
        with: withIn(compared.flatMap(moved)),
        baseline: movedPath(prev.baseline, from, to),
        plan: movedPath(prev.plan, from, to),
      }),
      replace: true,
    });
  };
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
    const isSwap = path !== null && compared.includes(path);
    void navigate({
      to: ".",
      search: (prev) => ({
        ...prev,
        with: isSwap ? withIn(swapped(compared, path, left.path)) : undefined,
        baseline:
          isSwap && prev.baseline !== path
            ? (prev.baseline ?? left.path ?? undefined)
            : undefined,
        plan: undefined,
      }),
      replace: true,
    });
  }, [path, document, compared, navigate]);
}
