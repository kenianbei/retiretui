import type { FieldView, Place } from "@wasm/retiretui_wasm.js";

/** What a domain's page holds open, in its address. */
export interface PlanSearch {
  /** The highlighted item's plan index. */
  item?: number;
  /** The item open in the form: its plan index, or a new one. */
  edit?: number | "new";
  /** The field the form is scrolled to. */
  field?: string;
}

function indexOf(value: unknown): number | undefined {
  const index = typeof value === "string" ? Number(value) : value;
  return typeof index === "number" && Number.isInteger(index) && index >= 0
    ? index
    : undefined;
}

/** A domain page's search params from whatever the address holds. */
export function planSearch(search: Record<string, unknown>): PlanSearch {
  const item = indexOf(search.item);
  const edit = search.edit === "new" ? "new" : indexOf(search.edit);
  const field = typeof search.field === "string" ? search.field : undefined;
  return {
    ...(item !== undefined && { item }),
    ...(edit !== undefined && { edit }),
    ...(field !== undefined && { field }),
  };
}

/** The DOM id a field's control has, so a link can land on it. */
export function fieldId(view: Pick<FieldView, "key" | "place">): string {
  return `field-${view.key}${view.place === null ? "" : `-${String(view.place)}`}`;
}

/**
 * Scrolls to and focuses `key`'s control within `root`, a list's first row
 * where it is one, answering whether there was one.
 */
export function landOnField(root: ParentNode, key: string): boolean {
  const shown = root.querySelector<HTMLElement>(
    `[id^="${fieldId({ key, place: null })}"]`,
  );
  if (!shown) return false;
  shown.scrollIntoView({ block: "center" });
  shown.focus();
  return true;
}

/** Where an issue's link lands: its item's form, scrolled to its field. */
export function placeSearch(place: Place): {
  page: string;
  search: PlanSearch;
} {
  const index = place.index ?? 0;
  return {
    page: place.domain,
    search: {
      ...(place.index !== null && { item: index }),
      edit: index,
      ...(place.field !== null && { field: place.field }),
    },
  };
}
