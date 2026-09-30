import { useId, type ReactNode } from "react";

/**
 * A chart, or a chart's table, as a section of its page: its title as its
 * heading, what it is in beneath, anything that changes what it shows at
 * the heading's end, and the plot, which takes its shape from the
 * section's width.
 */
export function ChartSection({
  title,
  unit,
  controls,
  children,
}: {
  title: string;
  unit: string;
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
          <p className="text-muted-foreground text-xs">{unit}</p>
        </div>
        {controls}
      </div>
      {children}
    </section>
  );
}
