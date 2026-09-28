import { useEffect } from "react";

import { isHeld } from "@/lib/keys";
import { TABS } from "@/nav";
import { useGoTo } from "@/shell/go";

/** Opens the palette, the key sheet, or a tab, wherever nothing else holds the key. */
export function useShellKeys(openPalette: () => void, openKeys: () => void) {
  const goTo = useGoTo();
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if (event.defaultPrevented || isHeld(event.target)) return;
      const isCommand = event.ctrlKey || event.metaKey;
      if (isCommand && event.key.toLowerCase() === "k") {
        event.preventDefault();
        openPalette();
        return;
      }
      if (isCommand || event.altKey) return;
      if (event.key === "?") {
        event.preventDefault();
        openKeys();
        return;
      }
      const tab = /^[1-9]$/.test(event.key)
        ? TABS[Number(event.key) - 1]
        : undefined;
      if (!tab) return;
      event.preventDefault();
      goTo(tab);
    };
    window.addEventListener("keydown", press);
    return () => {
      window.removeEventListener("keydown", press);
    };
  }, [openPalette, openKeys, goTo]);
}
