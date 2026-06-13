import { test, expect, describe } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// Custom-property definitions (`--name: value;`) declared in a theme file.
function definedTokens(css: string): Set<string> {
  const tokens = new Set<string>();
  for (const match of css.matchAll(/^\s*(--[a-z0-9-]+)\s*:/gim)) {
    tokens.add(match[1]);
  }
  return tokens;
}

describe("theme tokens", () => {
  // Spec: ui-shell — "Light mode renders from token counterparts".
  test("light mode renders from token counterparts", () => {
    const dir = import.meta.dir;
    const dark = definedTokens(readFileSync(join(dir, "dark-2026.css"), "utf8"));
    const light = definedTokens(readFileSync(join(dir, "light-2026.css"), "utf8"));

    const missingInLight = [...dark].filter((token) => !light.has(token));
    const missingInDark = [...light].filter((token) => !dark.has(token));

    expect(missingInLight).toEqual([]);
    expect(missingInDark).toEqual([]);
  });
});
