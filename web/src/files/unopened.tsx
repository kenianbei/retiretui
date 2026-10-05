import type { OpenFailure } from "@wasm/retiretui_wasm.js";
import { Download, Upload } from "lucide-react";
import { useLayoutEffect, useRef } from "react";

import { Button } from "@/components/ui/button";
import { MarginNote } from "@/components/margin-note";
import { useFileActions } from "@/files/actions";
import { cn } from "@/lib/utils";
import { nameOf } from "@/workspace";

interface UnopenedProps {
  /** The file asked for. */
  path: string;
  failure: OpenFailure;
}

/**
 * A plan that would not open: why, the file that failed to download or
 * replace, and its text where the failure is in it.
 */
export function Unopened({ path, failure }: UnopenedProps) {
  const actions = useFileActions();
  const { written } = failure;
  const failing = written?.file ?? path;
  return (
    <MarginNote zone="shortfall">
      <div role="alert">
        <p className="font-semibold">{nameOf(path)} could not be opened</p>
        <p className="text-sm break-words">{failure.headline}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            actions.download(failing);
          }}
        >
          <Download aria-hidden />
          Download {nameOf(failing)}
        </Button>
        <Button variant="outline" size="sm" onClick={actions.upload}>
          <Upload aria-hidden />
          Upload a fixed copy
        </Button>
      </div>
      {written && (
        <Written
          name={nameOf(failing)}
          text={written.text}
          line={written.line}
        />
      )}
    </MarginNote>
  );
}

interface WrittenProps {
  name: string;
  text: string;
  /** The line the failure is on, from 1. */
  line: number | null;
}

/** A file's text as written, numbered, `line` marked and scrolled to. */
function Written({ name, text, line }: WrittenProps) {
  const box = useRef<HTMLDivElement>(null);
  const marked = useRef<HTMLSpanElement>(null);
  const lines = text.replace(/\n$/, "").split("\n");

  // biome-ignore lint/correctness/useExhaustiveDependencies: the marked line is scrolled to again when the line or the text changes
  useLayoutEffect(() => {
    const [within, at] = [box.current, marked.current];
    if (!within || !at) return;
    within.scrollTop = at.offsetTop - within.clientHeight / 2;
  }, [line, text]);

  return (
    <div
      ref={box}
      role="region"
      aria-label={`${name} as written`}
      tabIndex={0}
      className="bg-card relative max-h-80 overflow-auto rounded-md border"
    >
      <pre className="py-2 font-mono text-sm">
        <code className="grid min-w-max">
          {lines.map((written, at) => {
            const number = at + 1;
            const isMarked = number === line;
            return (
              <span
                key={number}
                ref={isMarked ? marked : undefined}
                className={cn(
                  "grid grid-cols-[3.5rem_1fr] pr-4",
                  isMarked && "bg-destructive/10 font-semibold",
                )}
              >
                <span
                  className={cn(
                    "pr-4 text-right tabular-nums select-none",
                    isMarked ? "text-destructive" : "text-muted-foreground",
                  )}
                >
                  {number}
                </span>
                <span className="whitespace-pre">
                  {written || " "}
                  {isMarked && <span className="sr-only"> (the error)</span>}
                </span>
              </span>
            );
          })}
        </code>
      </pre>
    </div>
  );
}
