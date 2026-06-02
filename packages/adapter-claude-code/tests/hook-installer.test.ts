import { test, expect, describe } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { installHooks, uninstallHooks } from "../src/hook-installer";

const FIXTURES = path.join(import.meta.dir, "fixtures");
const NOTIFY_CMD = "bun /home/user/.athing/notify.mjs";

const noop = () => {};
const logger = { debug: noop, info: noop, warn: noop, error: noop };

function tempSettings(fixtureName: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-test-"));
  const dest = path.join(dir, "settings.json");
  fs.copyFileSync(path.join(FIXTURES, fixtureName), dest);
  return dest;
}

function readJson(p: string): Settings {
  return JSON.parse(fs.readFileSync(p, "utf8")) as Settings;
}

type HookEntry = { matcher: string; hooks: Array<{ type: string; command: string }> };
type Settings = { hooks?: Record<string, HookEntry[]> };

function hookCommands(settings: Settings, event: string): string[] {
  return (settings.hooks?.[event] ?? []).flatMap((e) => e.hooks.map((h) => h.command));
}

describe("installHooks", () => {
  test("adds all hook events when settings is empty", () => {
    const p = tempSettings("settings-empty.json");
    installHooks(NOTIFY_CMD, logger, p);
    const result = readJson(p);
    const events = [
      "SessionStart",
      "UserPromptSubmit",
      "PostToolUse",
      "PermissionRequest",
      "Stop",
      "SessionEnd",
    ];
    for (const event of events) {
      expect(hookCommands(result, event)).toContain(NOTIFY_CMD);
    }
  });

  test("PostToolUse entry has matcher *", () => {
    const p = tempSettings("settings-empty.json");
    installHooks(NOTIFY_CMD, logger, p);
    const result = readJson(p);
    const entry = result.hooks?.["PostToolUse"]?.find((e) =>
      e.hooks.some((h) => h.command === NOTIFY_CMD),
    );
    expect(entry?.matcher).toBe("*");
  });

  test("non-PostToolUse entries have empty matcher", () => {
    const p = tempSettings("settings-empty.json");
    installHooks(NOTIFY_CMD, logger, p);
    const result = readJson(p);
    for (const event of [
      "SessionStart",
      "UserPromptSubmit",
      "PermissionRequest",
      "Stop",
      "SessionEnd",
    ]) {
      const entry = result.hooks?.[event]?.find((e) =>
        e.hooks.some((h) => h.command === NOTIFY_CMD),
      );
      expect(entry?.matcher).toBe("");
    }
  });

  test("does not add duplicate entries when all hooks already installed", () => {
    const p = tempSettings("settings-all-hooks.json");
    installHooks(NOTIFY_CMD, logger, p);
    const result = readJson(p);
    for (const event of ["SessionStart", "Stop"]) {
      const count = hookCommands(result, event).filter((c) => c === NOTIFY_CMD).length;
      expect(count).toBe(1);
    }
  });

  test("only adds missing events when partially installed", () => {
    const p = tempSettings("settings-partial-hooks.json");
    installHooks(NOTIFY_CMD, logger, p);
    const result = readJson(p);
    const missing = ["UserPromptSubmit", "PermissionRequest", "Stop", "SessionEnd"];
    for (const event of missing) {
      expect(hookCommands(result, event)).toContain(NOTIFY_CMD);
    }
    const alreadyPresent = ["SessionStart", "PostToolUse"];
    for (const event of alreadyPresent) {
      const count = hookCommands(result, event).filter((c) => c === NOTIFY_CMD).length;
      expect(count).toBe(1);
    }
  });

  test("creates a backup file before writing", () => {
    const p = tempSettings("settings-empty.json");
    installHooks(NOTIFY_CMD, logger, p);
    const dir = path.dirname(p);
    const backups = fs.readdirSync(dir).filter((f) => f.includes("athing-backup"));
    expect(backups.length).toBe(1);
  });
});

describe("uninstallHooks", () => {
  test("removes athing-notify entries from all events", () => {
    const p = tempSettings("settings-all-hooks.json");
    uninstallHooks(logger, p);
    const result = readJson(p);
    for (const event of [
      "SessionStart",
      "UserPromptSubmit",
      "PostToolUse",
      "PermissionRequest",
      "Stop",
      "SessionEnd",
    ]) {
      expect(hookCommands(result, event)).not.toContain(NOTIFY_CMD);
    }
  });

  test("preserves non-athing-notify hooks", () => {
    const p = tempSettings("settings-mixed-hooks.json");
    uninstallHooks(logger, p);
    const result = readJson(p);
    expect(hookCommands(result, "SessionStart")).toContain("some-other-tool");
  });

  test("creates a backup file before writing", () => {
    const p = tempSettings("settings-all-hooks.json");
    uninstallHooks(logger, p);
    const dir = path.dirname(p);
    const backups = fs.readdirSync(dir).filter((f) => f.includes("athing-backup"));
    expect(backups.length).toBe(1);
  });

  test("does not write when no athing-notify entries present", () => {
    const p = tempSettings("settings-other-hooks.json");
    const before = fs.readFileSync(p, "utf8");
    uninstallHooks(logger, p);
    const after = fs.readFileSync(p, "utf8");
    expect(after).toBe(before);
  });

  test("does not write when settings has no hooks key", () => {
    const p = tempSettings("settings-empty.json");
    const before = fs.readFileSync(p, "utf8");
    uninstallHooks(logger, p);
    const after = fs.readFileSync(p, "utf8");
    expect(after).toBe(before);
  });
});
