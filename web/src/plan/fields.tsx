import type {
  Editor,
  FieldView,
  Offer,
  OperandView,
} from "@wasm/retiretui_wasm.js";
import { useId, useState, type ChangeEvent, type ReactNode } from "react";

import { INPUT, cn } from "@/lib/utils";
import { fieldId } from "@/plan/search";

/** What a field's control changes, and whom it tells. */
export interface FieldProps {
  view: FieldView;
  editor: Pick<Editor, "set" | "tick" | "setTrigger">;
  /** Re-reads the editor after it changed. */
  changed: () => void;
  /** Says which field is being typed into; `null` once none is. */
  focus: (key: string | null) => void;
}

/** A pick with more offers than this is searched rather than scrolled. */
const LONG_LIST = 40;

/** A rate's slider reaches this far; a higher one is still typed. */
const RATE_CEILING = 0.15;
const RATE_STEP = 0.0025;
const SHARE_STEP = 0.05;
const PERCENT = 100;

/**
 * An input's props that show `shown`, or `typed` once it has the keyboard,
 * and keep what is typed into it until it loses the keyboard, handing each
 * change to `enter`.
 */
function useTyping(
  shown: string,
  typed: string,
  enter: (text: string) => void,
) {
  const [typing, setTyping] = useState<string | null>(null);
  return {
    value: typing ?? shown,
    onFocus: () => {
      setTyping(typed);
    },
    onChange: (event: ChangeEvent<HTMLInputElement>) => {
      setTyping(event.target.value);
      enter(event.target.value);
    },
    onBlur: () => {
      setTyping(null);
    },
  };
}

/** A label, its help, what is wrong, then the control, tied together. */
function Labelled({
  view,
  children,
}: {
  view: FieldView;
  children: (described: string | undefined) => ReactNode;
}) {
  const id = useId();
  const problem = view.complaint ?? view.issue;
  const described =
    [view.help && `${id}-help`, problem && `${id}-problem`]
      .filter(Boolean)
      .join(" ") || undefined;
  return (
    <div className="grid gap-1.5">
      <label htmlFor={fieldId(view)} className="text-sm font-medium">
        {view.label}
      </label>
      {view.help && (
        <p id={`${id}-help`} className="text-muted-foreground text-xs">
          {view.help}
        </p>
      )}
      {problem && (
        <p id={`${id}-problem`} className="text-destructive text-sm">
          {problem}
        </p>
      )}
      {children(described)}
    </div>
  );
}

/** Typed text: what is typed is kept while the field has the keyboard. */
function TextField({
  view,
  editor,
  changed,
  focus,
  described,
}: FieldProps & { described: string | undefined }) {
  const typing = useTyping(view.text, view.typed, (text) => {
    editor.set(view.key, view.place ?? undefined, text);
    changed();
  });
  const mode =
    view.control === "whole"
      ? "numeric"
      : ["money", "listed", "rate", "share"].includes(view.control)
        ? "decimal"
        : undefined;
  return (
    <input
      id={fieldId(view)}
      className={INPUT}
      inputMode={mode}
      placeholder={view.blank}
      aria-describedby={described}
      aria-invalid={Boolean(view.complaint ?? view.issue) || undefined}
      {...typing}
      onFocus={() => {
        typing.onFocus();
        focus(view.key);
      }}
      onBlur={() => {
        typing.onBlur();
        focus(null);
      }}
    />
  );
}

/** A rate or a share on a slider, beside the exact text. */
function RateField(props: FieldProps & { described: string | undefined }) {
  const { view, editor, changed } = props;
  const isShare = view.control === "share";
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        aria-label={`${view.label}, on a slider`}
        className="accent-primary w-1/2"
        min={0}
        max={isShare ? 1 : RATE_CEILING}
        step={isShare ? SHARE_STEP : RATE_STEP}
        value={view.number ?? 0}
        onChange={(event) => {
          const percent = Number(event.target.value) * PERCENT;
          editor.set(
            view.key,
            view.place ?? undefined,
            `${percent.toFixed(2)}%`,
          );
          changed();
        }}
      />
      <div className="flex-1">
        <TextField {...props} />
      </div>
    </div>
  );
}

/** A pick over offers: a select, or a searched list where it is long. */
function PickField({
  id,
  offers,
  value,
  described,
  isInvalid,
  pick,
}: {
  id: string;
  offers: Offer[];
  value: string;
  described?: string;
  isInvalid?: boolean;
  pick: (value: string) => void;
}) {
  const listId = useId();
  const labelOf = (held: string) =>
    offers.find((offer) => offer.value === held)?.label ?? held;
  const shown = labelOf(value);
  const typing = useTyping(shown, shown, (typed) => {
    const offer = offers.find((each) => each.label === typed);
    if (offer) pick(offer.value);
  });
  if (offers.length > LONG_LIST) {
    return (
      <>
        <input
          id={id}
          className={INPUT}
          list={listId}
          aria-describedby={described}
          aria-invalid={isInvalid || undefined}
          {...typing}
        />
        <datalist id={listId}>
          {offers.map((offer) => (
            <option key={offer.value} value={offer.label} />
          ))}
        </datalist>
      </>
    );
  }
  return (
    <select
      id={id}
      className={INPUT}
      aria-describedby={described}
      aria-invalid={isInvalid || undefined}
      value={value}
      onChange={(event) => {
        pick(event.target.value);
      }}
    >
      {offers.map((offer) => (
        <option key={offer.value} value={offer.value}>
          {offer.label}
        </option>
      ))}
    </select>
  );
}

/** Words a trigger's date operand shows as a hint rather than beside it. */
const DATE_HINT = "yyyy-mm-dd";

/** A typed operand of a trigger, kept as typed while it has the keyboard. */
function OperandText({
  operand,
  set,
}: {
  operand: OperandView;
  set: (text: string) => void;
}) {
  const typing = useTyping(operand.text, operand.text, set);
  return (
    <input
      aria-label={operand.help}
      title={operand.help}
      className={cn(INPUT, operand.after === DATE_HINT ? "w-32" : "w-20")}
      placeholder={operand.after === DATE_HINT ? DATE_HINT : ""}
      {...typing}
    />
  );
}

/** A trigger: its kind, then the sentence of that kind's operands. */
function TriggerField({
  view,
  editor,
  changed,
  described,
}: FieldProps & { described: string | undefined }) {
  const trigger = view.trigger;
  if (!trigger) return null;
  const set = (part: string, text: string) => {
    editor.setTrigger(view.key, part, text);
    changed();
  };
  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="w-36">
        <PickField
          id={fieldId(view)}
          offers={trigger.bases}
          value={trigger.basis}
          described={described}
          isInvalid={Boolean(view.complaint ?? view.issue)}
          pick={(basis) => {
            set("basis", basis);
          }}
        />
      </div>
      {trigger.operands.map((operand) => (
        <span key={operand.key} className="flex items-center gap-2 text-sm">
          {operand.before.trim() && <span>{operand.before.trim()}</span>}
          {operand.offers ? (
            <PickField
              id={`${fieldId(view)}-${operand.key}`}
              offers={operand.offers}
              value={operand.text}
              pick={(value) => {
                set(operand.key, value);
              }}
            />
          ) : (
            <OperandText
              operand={operand}
              set={(text) => {
                set(operand.key, text);
              }}
            />
          )}
          {operand.after.trim() && operand.after !== DATE_HINT && (
            <span>{operand.after.trim()}</span>
          )}
        </span>
      ))}
    </div>
  );
}

/** One field on show, entered as its control says. */
export function Field(props: FieldProps) {
  const { view, editor, changed } = props;
  if (view.control === "flag" || view.control === "presence") {
    return (
      <label
        htmlFor={fieldId(view)}
        className="flex items-start gap-3 text-sm font-medium"
      >
        <input
          id={fieldId(view)}
          type="checkbox"
          className="accent-primary mt-0.5 size-4"
          checked={view.is_ticked}
          onChange={(event) => {
            editor.tick(view.key, event.target.checked);
            changed();
          }}
        />
        <span className="grid gap-1">
          {view.label}
          {view.help && (
            <span className="text-muted-foreground text-xs font-normal">
              {view.help}
            </span>
          )}
        </span>
      </label>
    );
  }
  return (
    <Labelled view={view}>
      {(described) => {
        switch (view.control) {
          case "choice":
          case "order":
            return (
              <PickField
                id={fieldId(view)}
                offers={view.offers}
                value={view.text}
                described={described}
                isInvalid={Boolean(view.complaint ?? view.issue)}
                pick={(value) => {
                  editor.set(view.key, view.place ?? undefined, value);
                  changed();
                }}
              />
            );
          case "trigger":
            return <TriggerField {...props} described={described} />;
          case "rate":
          case "share":
            return <RateField {...props} described={described} />;
          case "remainder":
            return (
              <output
                id={fieldId(view)}
                aria-describedby={described}
                className={cn(
                  "text-sm tabular-nums",
                  view.is_exceeded && "text-destructive font-semibold",
                )}
              >
                {view.text}
              </output>
            );
          default:
            return <TextField {...props} described={described} />;
        }
      }}
    </Labelled>
  );
}
