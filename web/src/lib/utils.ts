import { clsx, type ClassValue } from "clsx";
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
