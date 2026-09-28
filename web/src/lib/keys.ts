/**
 * Whether a key pressed at `target` belongs to it: a field, or anything
 * in a form, question or menu standing over the page, which keeps every key.
 */
export function isHeld(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
      target.closest('[role="dialog"], [role="alertdialog"], [role="menu"]') !==
        null)
  );
}
