import { describe, expect, it } from "vitest";

import { builtOn, scenariosOver } from "@/files/chain";

const BASES: Record<string, string> = {
  "/ladder.toml": "/plan.toml",
  "/ladder-late.toml": "/ladder.toml",
  "/claims.toml": "/plan.toml",
  "/elsewhere.toml": "/other.toml",
};
const PATHS = ["/plan.toml", "/other.toml", ...Object.keys(BASES)];
const baseAt = (path: string) => BASES[path];

describe("a scenario chain", () => {
  it("names the scenarios over a file, and everything built on it", () => {
    expect(scenariosOver("/plan.toml", PATHS, baseAt)).toEqual([
      "/ladder.toml",
      "/claims.toml",
    ]);
    expect(builtOn("/plan.toml", PATHS, baseAt).sort()).toEqual([
      "/claims.toml",
      "/ladder-late.toml",
      "/ladder.toml",
    ]);
    expect(builtOn("/claims.toml", PATHS, baseAt)).toEqual([]);
  });
});
