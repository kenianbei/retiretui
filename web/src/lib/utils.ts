import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** What a thrown refusal says. */
export function messageOf(thrown: unknown): string {
  return thrown instanceof Error ? thrown.message : String(thrown);
}

/** A text input's or a select's look, wherever a page draws one. */
export const INPUT =
  "border-input bg-background h-9 w-full min-w-0 rounded-md border px-3 text-base md:text-sm aria-invalid:border-destructive";

/** `items` in runs of neighbours under the same heading, or under none. */
export function gathered<T extends { group?: string | null }>(
  items: readonly T[],
): { group: string | null; items: T[] }[] {
  const runs: { group: string | null; items: T[] }[] = [];
  for (const item of items) {
    const group = item.group ?? null;
    const last = runs.at(-1);
    if (last?.group === group) last.items.push(item);
    else runs.push({ group, items: [item] });
  }
  return runs;
}
