import type { ErrorComponentProps } from "@tanstack/react-router";
import { RotateCw } from "lucide-react";

import { MarginNote } from "@/components/margin-note";
import { Button } from "@/components/ui/button";
import { messageOf } from "@/lib/utils";
import { reportUrl } from "@/links";

/** What stands in for a page that threw while it was drawn. */
export function Failed({ error }: ErrorComponentProps) {
  return (
    <section className="max-w-prose space-y-4">
      <h1 className="text-2xl font-semibold tracking-tight">
        This page stopped working
      </h1>
      <MarginNote zone="shortfall" role="alert">
        <p className="text-sm break-words">{messageOf(error)}</p>
      </MarginNote>
      <p className="text-muted-foreground">
        The plans you have saved are still in this browser. Reloading starts the
        page again, without any edits not yet saved.
      </p>
      <div className="flex flex-wrap items-center gap-4">
        <Button
          onClick={() => {
            window.location.reload();
          }}
        >
          <RotateCw aria-hidden />
          Reload
        </Button>
        <a
          href={reportUrl()}
          target="_blank"
          rel="noreferrer"
          className="text-primary underline underline-offset-4"
        >
          Report an issue
        </a>
      </div>
    </section>
  );
}
