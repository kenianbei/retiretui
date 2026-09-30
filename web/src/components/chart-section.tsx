import { useId, type ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * A chart as a section of its page: its title as its heading, what it is
 * in beneath, and the plot, which takes its shape from the section's width.
 */
export function ChartSection({
  title,
  unit,
  children,
  className,
}: {
  title: string;
  unit: string;
  children: ReactNode;
  className?: string;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className={cn(
        "bg-card @container min-w-0 space-y-3 rounded-xl border p-4 shadow-sm",
        className,
      )}
    >
      <div className="space-y-0.5">
        <h2 id={id} className="text-lg font-semibold">
          {title}
        </h2>
        <p className="text-muted-foreground text-xs">{unit}</p>
      </div>
      {children}
    </section>
  );
}
