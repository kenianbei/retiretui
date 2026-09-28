import { useRef } from "react";

/**
 * The browser's file dialog for files of `accept`, handing the one picked
 * to `pick`; its input is `element`, placed once wherever it is used.
 */
export function useFilePicker(
  accept: string,
  label: string,
  pick: (file: File) => Promise<void>,
) {
  const input = useRef<HTMLInputElement>(null);
  const element = (
    <input
      ref={input}
      type="file"
      accept={accept}
      aria-label={label}
      hidden
      onChange={(event) => {
        const file = event.target.files?.[0];
        event.target.value = "";
        if (file) void pick(file);
      }}
    />
  );
  return { open: () => input.current?.click(), element };
}
