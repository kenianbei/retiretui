import type { Step as SetupStep } from "@wasm/retiretui_wasm.js";

/** The last step, which reads every answer back before the plan is made. */
export const CHECK = "check";

/** Where the questions start; a step not asked sends on to the first that is. */
export const NEW_PLAN_START = "household";

/** Where the answers are kept while the questions are asked, in the tab's storage. */
const KEPT = "retiretui-app:new-plan";

/** What the address holds of a step besides its slug. */
export interface NewPlanSearch {
  /** The field a Change link lands on; Continue then goes back to the check. */
  field?: string;
}

export function newPlanSearch(search: Record<string, unknown>): NewPlanSearch {
  return typeof search.field === "string" ? { field: search.field } : {};
}

/** The steps with a question on show. */
export function stepsAsked(
  steps: readonly SetupStep[],
  shown: ReadonlySet<string>,
): SetupStep[] {
  return steps.filter((step) => step.keys.some((key) => shown.has(key)));
}

/** Where `step` stands among the steps asked and the check after them. */
export function around(asked: readonly SetupStep[], step: string) {
  const order = [...asked.map((each) => each.slug), CHECK];
  const at = order.indexOf(step);
  return {
    isAsked: at >= 0,
    place: at + 1,
    count: order.length,
    before: at > 0 ? order[at - 1] : undefined,
    after: order[at + 1],
  };
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
