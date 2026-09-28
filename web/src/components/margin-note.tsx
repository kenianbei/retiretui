import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

/** The zone a note reports: what stops the plan, what to heed, a note. */
export type Zone = "shortfall" | "caution" | "note";

const RULE: Record<Zone, string> = {
  shortfall: "border-destructive",
  caution: "border-warning",
  note: "border-primary",
};

/** A note in the margin: a rule down its left edge in its zone's colour. */
export function MarginNote({
  zone,
  className,
  ...props
}: ComponentProps<"div"> & { zone: Zone }) {
  return (
    <div
      className={cn("space-y-2 border-l-4 py-1 pl-4", RULE[zone], className)}
      {...props}
    />
  );
}
