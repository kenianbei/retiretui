import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const CSS = readFileSync(new URL("./index.css", import.meta.url), "utf8");
const TEXT = 4.5;
const MARK = 3;
/** How far apart, in CIE76 ΔE, a chart series stays from a zone colour. */
const APART = 20;
const SERIES = [1, 2, 3, 4, 5].map((n) => `chart-${String(n)}`);
const ZONES = ["destructive", "warning", "success"];

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
  ["success", "background"],
  ["warning", "background"],
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

function linear(hex: string): [number, number, number] {
  const [r = 0, g = 0, b = 0] = [1, 3, 5]
    .map((at) => parseInt(hex.slice(at, at + 2), 16) / 255)
    .map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return [r, g, b];
}

function luminance(hex: string): number {
  const [r, g, b] = linear(hex);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** A colour in CIELAB under D65, where distance is how far apart it looks. */
function lab(hex: string): [number, number, number] {
  const [r, g, b] = linear(hex);
  const [x, y, z] = [
    (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047,
    0.2126 * r + 0.7152 * g + 0.0722 * b,
    (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883,
  ].map((t) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116));
  return [
    116 * (y ?? 0) - 16,
    500 * ((x ?? 0) - (y ?? 0)),
    200 * ((y ?? 0) - (z ?? 0)),
  ];
}

function distance(one: string, other: string): number {
  const [a, b] = [lab(one), lab(other)];
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
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

  it.each(
    SERIES.flatMap((series) => ZONES.map((zone) => [series, zone] as const)),
  )("tells --%s from --%s", (series, zone) => {
    expect(distance(colour(series), colour(zone))).toBeGreaterThanOrEqual(
      APART,
    );
  });
});

it("measures how far apart colours look", () => {
  expect(distance("#e3b35a", "#e3b35a")).toBe(0);
  expect(distance("#000000", "#ffffff")).toBeCloseTo(100, 0);
});

it("measures contrast as WCAG does", () => {
  expect(contrast("#000000", "#ffffff")).toBeCloseTo(21);
  expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
});
