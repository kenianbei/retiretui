import { cn, INPUT } from "@/lib/utils";

interface LabelledSelectProps {
  label: string;
  value: string | undefined;
  options: readonly { value: string; label: string }[];
  onChange: (value: string) => void;
  /** The select's size, in place of a toolbar's. */
  className?: string;
  /** Its label shrinks in a row and its name never wraps. */
  isTight?: boolean;
}

/** A select named by the words beside it, as a page's toolbar draws one. */
export function LabelledSelect({
  label,
  value,
  options,
  onChange,
  className = "h-8 w-auto",
  isTight = false,
}: LabelledSelectProps) {
  return (
    <label
      className={cn("flex items-center gap-2 text-sm", isTight && "min-w-0")}
    >
      <span
        className={cn("text-muted-foreground", isTight && "whitespace-nowrap")}
      >
        {label}
      </span>
      <select
        value={value}
        onChange={(event) => {
          onChange(event.target.value);
        }}
        className={cn(INPUT, className)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
