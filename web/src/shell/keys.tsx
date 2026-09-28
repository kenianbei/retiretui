import { X } from "lucide-react";
import { Dialog } from "radix-ui";

import { Button } from "@/components/ui/button";
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
    <Dialog.Root open={isOpen} onOpenChange={setOpen}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/40" />
        <Dialog.Content
          aria-describedby={undefined}
          className="bg-background fixed top-1/2 left-1/2 z-50 w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-lg border p-4 shadow-xl"
        >
          <div className="mb-3 flex items-center justify-between">
            <Dialog.Title className="text-lg font-semibold">
              Keyboard shortcuts
            </Dialog.Title>
            <Dialog.Close asChild>
              <Button variant="ghost" size="icon" aria-label="Close">
                <X aria-hidden />
              </Button>
            </Dialog.Close>
          </div>
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
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
