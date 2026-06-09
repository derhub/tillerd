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

describe("gate-targeted hook install", () => {
  test("hook_installer_targets_the_notify_binary_for_every_event", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
    });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) {
      // The notify binary frames to the gate socket itself; the command is the
      // resolved binary path, not an inline curl.
      expect(commandsFor(s, event)).toContain(NOTIFY);
    }
  });

  test("hook_installer_installs_the_notify_command_for_every_event", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) {
      const cmds = commandsFor(s, event);
      expect(cmds).toContain(NOTIFY);
    }
  });

  test("hook_installer_idempotent: second install does not duplicate", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
    });
    await run(["install"], h.deps);
    const afterFirst = h.files.get("/agent/.claude/settings.json");
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    expect(h.out.join("\n")).toContain("hooks already installed");
    expect(h.files.get("/agent/.claude/settings.json")).toBe(afterFirst!);
  });

  test("uninstall removes notify hooks by marker", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
    });
    await run(["install"], h.deps);
    const code = await run(["uninstall"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) expect(commandsFor(s, event)).toHaveLength(0);
    expect(h.out.join("\n")).toContain("hooks uninstalled");
  });
});
