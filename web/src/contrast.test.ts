import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const CSS = readFileSync(new URL("./index.css", import.meta.url), "utf8");
const TEXT = 4.5;
const MARK = 3;

const TEXT_PAIRS = [
  ["foreground", "background"],
  ["card-foreground", "card"],
  ["popover-foreground", "popover"],
  ["primary-foreground", "primary"],
  ["secondary-foreground", "secondary"],
  ["accent-foreground", "accent"],
  ["muted-foreground", "background"],
  ["muted-foreground", "muted"],
  ["muted-foreground", "card"],
  ["primary", "background"],
  ["destructive", "background"],
  ["destructive", "card"],
  ["success", "card"],
  ["warning", "card"],
] as const;

const MARK_PAIRS: readonly (readonly [string, string])[] = [
  ["ring", "background"],
  ["input", "card"],
  ...[1, 2, 3, 4, 5].map((n) => [`chart-${String(n)}`, "card"] as const),
];

function tokens(block: string): Map<string, string> {
  const found = block.matchAll(/--([\w-]+):\s*(#[0-9a-f]{6});/g);
  return new Map([...found].map(([, name, hex]) => [name ?? "", hex ?? ""]));
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map(
    (at) => parseInt(hex.slice(at, at + 2), 16) / 255,
  );
  const [r, g, b] = channels.map((c) =>
    c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
}

function contrast(one: string, other: string): number {
  const [light, dark] = [luminance(one), luminance(other)].sort(
    (a, b) => b - a,
  );
  return ((light ?? 0) + 0.05) / ((dark ?? 0) + 0.05);
}

const light = tokens(CSS.slice(CSS.indexOf(":root"), CSS.indexOf("@media")));
const dark = tokens(CSS.slice(CSS.indexOf("@media"), CSS.indexOf("@theme")));

describe.each([
  ["light", light],
  ["dark", dark],
])("the %s palette", (_, palette) => {
  const colour = (name: string) => {
    const hex = palette.get(name);
    if (hex === undefined) throw new Error(`--${name} is not a hex colour`);
    return hex;
  };

  it.each(TEXT_PAIRS)("reads --%s on --%s", (text, ground) => {
    expect(contrast(colour(text), colour(ground))).toBeGreaterThanOrEqual(TEXT);
  });

  it.each(MARK_PAIRS)("sees --%s on --%s", (mark, ground) => {
    expect(contrast(colour(mark), colour(ground))).toBeGreaterThanOrEqual(MARK);
  });
});

it("measures contrast as WCAG does", () => {
  expect(contrast("#000000", "#ffffff")).toBeCloseTo(21);
  expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
});
