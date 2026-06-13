import { test, expect, describe } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

// Exempt per the ui-shell spec: vendored primitives (shadcn `ui/`) and the hardcoded
// terminal palette (theme-independent).
const EXEMPT_FILES = new Set(["TerminalPane.tsx", "DesktopTerminalPane.tsx"]);

function shellComponents(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "ui") continue; // vendored shadcn primitives
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...shellComponents(full));
    } else if (entry.name.endsWith(".tsx") && !EXEMPT_FILES.has(entry.name)) {
      files.push(full);
    }
  }
  return files;
}

// Ad-hoc styling values the shell must not carry: literal colors and raw-pixel
// arbitrary values. Token references (`var(--x)`, `[var(--x)]`) and content-relative
// `ch` measures resolve from tokens and are allowed.
const LITERAL_COLOR = /#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla)\(/;
const PX_ARBITRARY = /\[[^\]]*\d+px[^\]]*\]/;

describe("shell tokens", () => {
  // Spec: ui-shell — "Shell components use tokens only".
  test("shell components use tokens only", () => {
    const violations: string[] = [];
    for (const file of shellComponents(import.meta.dir)) {
      readFileSync(file, "utf8")
        .split("\n")
        .forEach((line, index) => {
          if (LITERAL_COLOR.test(line) || PX_ARBITRARY.test(line)) {
            violations.push(`${file}:${index + 1}: ${line.trim()}`);
          }
        });
    }
    expect(violations).toEqual([]);
  });
});
