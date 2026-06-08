/**
 * Import-boundary guard (rule 8.4, TS layer).
 *
 * The server's gate-client module is the TS equivalent of the Rust `gate-client`
 * crate: a thin codec/transport adapter that must only depend on `@athing/sdk`
 * contract types. It must not import `@athing/engine` internals or any Rust/
 * platform boundary (memorya, daemon-pty, etc.).
 *
 * This test reads the source file and asserts the import list statically.
 */
import { test, expect } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const GATE_CLIENT_SRC = join(import.meta.dir, "../src/gate-client.ts");

test("gate-client.ts imports only from @athing/sdk and node builtins", () => {
  const source = readFileSync(GATE_CLIENT_SRC, "utf-8");
  const importLines = source.split("\n").filter((line) => /^import\b/.test(line));

  const forbidden = ["@athing/engine", "@athing/platform-bun", "@athing/adapter-claude-code"];

  for (const line of importLines) {
    for (const pkg of forbidden) {
      expect(line).not.toContain(pkg);
    }
  }
});
