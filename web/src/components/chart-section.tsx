import { type ReactNode, useId } from "react";

/** A chart, or its table, as a section: its title the heading, its unit under it,
 * its controls at the heading's end; the section is the plot's container. */
export function ChartSection({
  title,
  unit,
  controls,
  children,
}: {
  title: string;
  /** What its figures are in; left out where the chart beside it says. */
  unit?: string;
  controls?: ReactNode;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className="bg-card @container min-w-0 space-y-3 rounded-xl border p-4 shadow-sm"
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="space-y-0.5">
          <h2 id={id} className="text-lg font-semibold">
            {title}
          </h2>
          {unit && <p className="text-muted-foreground text-xs">{unit}</p>}
        </div>
        {controls}
      </div>
      {children}
    </section>
  );
}
