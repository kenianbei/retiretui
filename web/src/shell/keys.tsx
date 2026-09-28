import { Dialog, DialogContent, DialogHeader } from "@/components/ui/dialog";
import { TABS } from "@/nav";

/** Every key the page answers, and what it does. */
const KEYS: readonly [string, string][] = [
  ["Ctrl K, ⌘ K", "Find a page, a plan or an action"],
  [`1 to ${String(TABS.length)}`, TABS.map((tab) => tab.title).join(", ")],
  ["?", "These keys"],
  ["Ctrl Z, ⌘ Z", "Undo"],
  ["Ctrl Shift Z, ⌘ Shift Z", "Redo"],
  ["Ctrl S, ⌘ S", "Save"],
  ["← →", "The year before or after, where a page shows one"],
  ["↑ ↓", "The row before or after, in a table"],
  ["Esc", "Close what stands over the page"],
];

/** Every key the page answers, listed. */
export function KeysSheet({
  isOpen,
  setOpen,
}: {
  isOpen: boolean;
  setOpen: (isOpen: boolean) => void;
}) {
  return (
    <Dialog open={isOpen} onOpenChange={setOpen}>
      <DialogContent
        aria-describedby={undefined}
        className="top-1/2 left-1/2 w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-lg border p-4"
      >
        <DialogHeader title="Keyboard shortcuts" className="mb-3" />
        <dl className="divide-y text-sm">
          {KEYS.map(([keys, does]) => (
            <div key={keys} className="flex justify-between gap-4 py-2">
              <dt>
                <kbd className="bg-muted rounded px-1.5 py-0.5 font-mono text-xs">
                  {keys}
                </kbd>
              </dt>
              <dd className="text-muted-foreground text-right">{does}</dd>
            </div>
          ))}
        </dl>
      </DialogContent>
    </Dialog>
  );
}
