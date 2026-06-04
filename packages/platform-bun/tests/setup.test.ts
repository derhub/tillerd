import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { setupFs, buildSetupContext } from "../src/setup";

const FIXTURES = path.join(import.meta.dir, "fixtures");

const dirs: string[] = [];
function tempDir(): string {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), "athing-setupfs-"));
  dirs.push(d);
  return d;
}

afterEach(() => {
  for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
});

describe("SetupFs — host filesystem capability", () => {
  test("readText returns null for a missing file, content when present", async () => {
    const p = path.join(tempDir(), "settings.json");
    expect(await setupFs.readText(p)).toBeNull();
    fs.writeFileSync(p, "hello", "utf8");
    expect(await setupFs.readText(p)).toBe("hello");
  });

  test("writeAtomic writes content and leaves no temp file", async () => {
    const dir = tempDir();
    const p = path.join(dir, "settings.json");
    await setupFs.writeAtomic(p, "data\n");
    expect(fs.readFileSync(p, "utf8")).toBe("data\n");
    expect(fs.readdirSync(dir).filter((f) => f.endsWith(".athing-tmp"))).toHaveLength(0);
  });

  test("writeAtomic creates missing parent directories", async () => {
    const p = path.join(tempDir(), "nested", "deep", "settings.json");
    await setupFs.writeAtomic(p, "x");
    expect(fs.existsSync(p)).toBe(true);
  });

  test("backup copies an existing file to a timestamped backup", async () => {
    const dir = tempDir();
    const p = path.join(dir, "settings.json");
    fs.writeFileSync(p, "before\n", "utf8");
    await setupFs.backup(p);
    const backups = fs.readdirSync(dir).filter((f) => f.includes("athing-backup"));
    expect(backups).toHaveLength(1);
    expect(fs.readFileSync(path.join(dir, backups[0]!), "utf8")).toBe("before\n");
  });

  test("backup is a no-op when the file is absent", async () => {
    const dir = tempDir();
    await setupFs.backup(path.join(dir, "settings.json"));
    expect(fs.readdirSync(dir)).toHaveLength(0);
  });

  test("backup-then-write preserves the prior file and records the new content", async () => {
    const dir = tempDir();
    const p = path.join(dir, "settings.json");
    const prior = fs.readFileSync(path.join(FIXTURES, "prior-settings.json"), "utf8");
    fs.writeFileSync(p, prior, "utf8");
    await setupFs.backup(p);
    await setupFs.writeAtomic(p, '{"updated":true}\n');
    expect(fs.readFileSync(p, "utf8")).toBe('{"updated":true}\n');
    const backup = fs.readdirSync(dir).find((f) => f.includes("athing-backup"))!;
    expect(fs.readFileSync(path.join(dir, backup), "utf8")).toBe(prior);
  });

  test("buildSetupContext carries the notify command, agent-home, logger, and fs", () => {
    const logger = {
      debug() {},
      info() {},
      warn() {},
      error() {},
      child() {
        return this;
      },
    };
    const ctx = buildSetupContext("/bin/notify", logger);
    expect(ctx.notifyCommand).toBe("/bin/notify");
    expect(ctx.agentHome.endsWith("/.claude")).toBe(true);
    expect(ctx.fs).toBe(setupFs);
    expect(ctx.logger).toBe(logger);
  });
});
