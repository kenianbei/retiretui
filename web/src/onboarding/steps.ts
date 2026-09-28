import type { SetupStep } from "@wasm/retiretui_wasm.js";

/** The last step, which reads every answer back before the plan is made. */
export const CHECK = "check";

/** Where the questions start; a step not asked sends on to the first that is. */
export const NEW_PLAN_START = "household";

/** Where the answers are kept while the questions are asked, in the tab's storage. */
const KEPT = "retiretui-app:new-plan";

/** What the address holds of a step besides its slug. */
export interface NewPlanSearch {
  /** The field to land on. */
  field?: string;
  /** Whether Continue goes back to the check, where a Change link came from. */
  isChanging?: boolean;
}

export function newPlanSearch(search: Record<string, unknown>): NewPlanSearch {
  return {
    ...(typeof search.field === "string" && { field: search.field }),
    ...(search.isChanging === true && { isChanging: true }),
  };
}

/** The slugs of the steps with a question on show, then the check. */
export function stepsShown(
  steps: readonly SetupStep[],
  shown: ReadonlySet<string>,
): string[] {
  const asked = steps.filter((step) => step.keys.some((key) => shown.has(key)));
  return [...asked.map((step) => step.slug), CHECK];
}

/** The steps either side of `step`, where there are any. */
export function around(
  order: readonly string[],
  step: string,
): { before?: string; after?: string } {
  const at = order.indexOf(step);
  if (at < 0) return { after: order[0] };
  return { before: order[at - 1], after: order[at + 1] };
}

/** The answers kept from an earlier visit this tab, if any. */
export function keptAnswers(storage: Storage): string | null {
  try {
    return storage.getItem(KEPT);
  } catch {
    return null;
  }
}

/** Keeps `answers` for a reload; `null` forgets them. */
export function keepAnswers(storage: Storage, answers: string | null) {
  try {
    if (answers === null) storage.removeItem(KEPT);
    else storage.setItem(KEPT, answers);
  } catch {
    // The answers still stand on this page; only a reload loses them.
  }
}
