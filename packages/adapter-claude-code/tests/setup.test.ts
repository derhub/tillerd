import { test, expect, describe } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import type { Logger, SetupContext, SetupFs } from "@athing/sdk";
import { setup } from "../src/setup";

const FIXTURES = path.join(import.meta.dir, "fixtures");
const AGENT_HOME = "/home/user/.claude";
const SETTINGS = `${AGENT_HOME}/settings.json`;
const NOTIFY = "/proj/bin/athing-notify";
const FIXTURE_NOTIFY = "/fixtures/bin/athing-notify";

const noop = () => {};
const logger: Logger = { debug: noop, info: noop, warn: noop, error: noop };

/** Seed the in-memory filesystem with a hand-authored settings fixture. */
function fakeFs(fixture?: string) {
  const files = new Map<string, string>();
  if (fixture) files.set(SETTINGS, fs.readFileSync(path.join(FIXTURES, fixture), "utf8"));
  const seq: string[] = [];
  const cap: SetupFs = {
    async readText(p) {
      return files.has(p) ? files.get(p)! : null;
    },
    async writeAtomic(p, text) {
      files.set(p, text);
      seq.push(`write:${p}`);
    },
    async backup(p) {
      if (files.has(p)) seq.push(`backup:${p}`);
    },
    async exists(p) {
      return files.has(p);
    },
  };
  return { fs: cap, files, seq };
}

function ctx(cap: SetupFs): SetupContext {
  return { notifyCommand: NOTIFY, agentHome: AGENT_HOME, logger, fs: cap };
}

type Settings = { hooks?: Record<string, Array<{ matcher: string; hooks: Array<{ command: string }> }>> };
function read(files: Map<string, string>): Settings {
  return JSON.parse(files.get(SETTINGS)!) as Settings;
}
function commands(s: Settings, event: string): string[] {
  return (s.hooks?.[event] ?? []).flatMap((e) => e.hooks.map((h) => h.command));
}

const EVENTS = ["SessionStart", "UserPromptSubmit", "PostToolUse", "PermissionRequest", "Stop", "SessionEnd"];

describe("setup.install", () => {
  test("adds all hook events when settings empty", async () => {
    const f = fakeFs("empty-settings.json");
    await setup.install(ctx(f.fs));
    const s = read(f.files);
    for (const event of EVENTS) expect(commands(s, event)).toContain(NOTIFY);
  });

  test("writes under the supplied agent-home", async () => {
    const f = fakeFs("empty-settings.json");
    await setup.install(ctx(f.fs));
    expect(f.files.has(SETTINGS)).toBe(true);
  });

  test("PostToolUse uses matcher *, others empty", async () => {
    const f = fakeFs("empty-settings.json");
    await setup.install(ctx(f.fs));
    const s = read(f.files);
    expect(s.hooks!["PostToolUse"]!.find((e) => e.hooks.some((h) => h.command === NOTIFY))!.matcher).toBe("*");
    for (const event of ["SessionStart", "Stop"]) {
      expect(s.hooks![event]!.find((e) => e.hooks.some((h) => h.command === NOTIFY))!.matcher).toBe("");
    }
  });

  test("backs up the prior file before writing", async () => {
    const f = fakeFs("settings-with-theme.json");
    await setup.install(ctx(f.fs));
    expect(f.seq).toEqual([`backup:${SETTINGS}`, `write:${SETTINGS}`]);
  });

  test("preserves unrelated settings keys", async () => {
    const f = fakeFs("settings-with-theme.json");
    await setup.install(ctx(f.fs));
    expect((read(f.files) as { theme?: string }).theme).toBe("dark");
  });

  test("idempotent: already installed performs no write", async () => {
    const f = fakeFs("empty-settings.json");
    await setup.install(ctx(f.fs));
    const after = f.seq.length;
    await setup.install(ctx(f.fs));
    expect(f.seq.length).toBe(after);
  });

  test("only adds missing events when partially installed", async () => {
    const f = fakeFs("partial-hooks.json");
    await setup.install(ctx(f.fs));
    const s = read(f.files);
    for (const event of ["UserPromptSubmit", "PermissionRequest", "Stop", "SessionEnd"]) {
      expect(commands(s, event)).toContain(NOTIFY);
    }
    // pre-existing marker entries are left untouched
    expect(commands(s, "SessionStart")).toEqual([FIXTURE_NOTIFY]);
  });
});

describe("setup.uninstall", () => {
  test("removes athing-notify entries from all events", async () => {
    const f = fakeFs("empty-settings.json");
    await setup.install(ctx(f.fs));
    await setup.uninstall(ctx(f.fs));
    const s = read(f.files);
    for (const event of EVENTS) expect(commands(s, event)).not.toContain(NOTIFY);
  });

  test("preserves non-athing-notify hooks", async () => {
    const f = fakeFs("mixed-hooks.json");
    await setup.uninstall(ctx(f.fs));
    const s = read(f.files);
    expect(commands(s, "SessionStart")).toContain("some-other-tool");
    expect(commands(s, "SessionStart")).not.toContain(FIXTURE_NOTIFY);
  });

  test("idempotent: nothing to remove performs no write", async () => {
    const f = fakeFs("other-hooks.json");
    await setup.uninstall(ctx(f.fs));
    expect(f.seq).toHaveLength(0);
  });

  test("no settings file performs no write", async () => {
    const f = fakeFs();
    await setup.uninstall(ctx(f.fs));
    expect(f.seq).toHaveLength(0);
  });
});
