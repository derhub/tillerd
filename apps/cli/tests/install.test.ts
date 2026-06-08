import { test, expect, describe } from "bun:test";
import { run } from "../src/cli";
import { harness, NOTIFY, commandsFor, settings } from "./helpers";

const EVENTS = [
  "SessionStart",
  "UserPromptSubmit",
  "PostToolUse",
  "PermissionRequest",
  "Stop",
  "SessionEnd",
];

describe("install", () => {
  test("writes hooks to the settings file on a fresh environment", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) expect(commandsFor(s, event)).toContain(NOTIFY);
    expect(h.out.join("\n")).toContain("installed hooks:");
  });

  test("idempotent: second install reports already installed and writes nothing new", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false });
    await run(["install"], h.deps);
    const afterFirst = h.files.get("/agent/.claude/settings.json");
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    expect(h.out.join("\n")).toContain("hooks already installed");
    expect(h.files.get("/agent/.claude/settings.json")).toBe(afterFirst!);
  });

  test("declined confirmation on a TTY makes no changes and exits non-zero", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: true, confirmResult: false });
    const code = await run(["install"], h.deps);
    expect(code).not.toBe(0);
    expect(h.confirmCalls).toBe(1);
    expect(settings(h.files).hooks).toBeUndefined();
  });

  test("confirmed on a TTY installs", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: true, confirmResult: true });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    expect(h.confirmCalls).toBe(1);
  });

  test("--yes skips the prompt even on a TTY", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: true, confirmResult: false });
    const code = await run(["install", "--yes"], h.deps);
    expect(code).toBe(0);
    expect(h.confirmCalls).toBe(0);
  });

  test("non-TTY never prompts", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false });
    await run(["install"], h.deps);
    expect(h.confirmCalls).toBe(0);
  });

  test("reports failure when notify command cannot be resolved", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
      resolveNotify: () => {
        throw new Error("notify client not found");
      },
    });
    const code = await run(["install"], h.deps);
    expect(code).not.toBe(0);
    expect(h.err.join("\n")).toContain("install failed");
  });
});

describe("uninstall", () => {
  test("removes managed hooks but preserves unrelated hook entries", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false });
    await run(["install"], h.deps);
    const code = await run(["uninstall"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) expect(commandsFor(s, event)).not.toContain(NOTIFY);
    expect(h.out.join("\n")).toContain("hooks uninstalled");
  });

  test("preserves a non-managed hook on uninstall", async () => {
    const h = harness({ fixture: "settings-other-hook.json", isTTY: false });
    await run(["uninstall"], h.deps);
    expect(commandsFor(settings(h.files), "SessionStart")).toContain("some-other-tool");
  });

  test("nothing to remove when no managed hooks present", async () => {
    const h = harness({ fixture: "settings-other-hook.json", isTTY: false });
    const code = await run(["uninstall"], h.deps);
    expect(code).toBe(0);
    expect(h.out.join("\n")).toContain("nothing to remove");
  });
});
