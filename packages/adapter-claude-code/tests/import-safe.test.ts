import { test, expect, describe } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import type { SetupContext, SetupFs } from "@tillerd/sdk";
import { setup, BINARY_RESOLUTION } from "../src/index";

const SRC = path.join(import.meta.dir, "..", "src");

const HOST_PRIMITIVE_PATTERNS = [
  /from\s+["']node:fs["']/,
  /from\s+["']node:os["']/,
  /from\s+["']node:path["']/,
  /require\(["']node:/,
  /\bprocess\.(env|cwd|platform)\b/,
  /\bhomedir\s*\(/,
];

const SRC_FILES = fs.readdirSync(SRC).filter((f) => f.endsWith(".ts"));

describe("adapter import-safety", () => {
  test.each(SRC_FILES)("%s reads no host primitive at module load or in functions", (file) => {
    const source = fs.readFileSync(path.join(SRC, file), "utf8");
    for (const pattern of HOST_PRIMITIVE_PATTERNS) {
      expect(source).not.toMatch(pattern);
    }
  });

  test("setup procedures reach the filesystem only through the injected capability", async () => {
    const files = new Map<string, string>();
    const capFs: SetupFs = {
      async readText(p) {
        return files.has(p) ? files.get(p)! : null;
      },
      async writeAtomic(p, text) {
        files.set(p, text);
      },
      async backup() {},
      async exists(p) {
        return files.has(p);
      },
    };
    const ctx: SetupContext = {
      notifyCommand: "/bin/tillerd-notify",
      agentHome: "/home/user/.claude",
      logger: {
        debug() {},
        info() {},
        warn() {},
        error() {},
        child() {
          return this;
        },
      },
      fs: capFs,
    };
    await setup.install(ctx);
    expect(files.has("/home/user/.claude/settings.json")).toBe(true);
  });

  test("binary-resolution policy is plain serializable data", () => {
    expect(() => JSON.parse(JSON.stringify(BINARY_RESOLUTION))).not.toThrow();
  });
});
