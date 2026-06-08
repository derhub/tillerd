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
  test("hook_installer_targets_gate_url_not_daemon_hooks_sock: gate curl embedded in hook when gate present", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
      gate: { gateUrl: "http://127.0.0.1:9999", sessionId: "sid1", sessionToken: "tok1" },
    });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) {
      const cmds = commandsFor(s, event);
      // Gate mode: command includes curl with bearer auth, not the notify path
      expect(cmds.some((c) => c.includes("Authorization: Bearer"))).toBe(true);
      expect(cmds.some((c) => c.includes("ATHING_GATE_URL"))).toBe(true);
    }
  });

  test("hook_installer_degrades_to_daemon_when_gate_absent: uses notify command when no gate", async () => {
    const h = harness({ fixture: "empty-settings.json", isTTY: false, gate: {} });
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) {
      const cmds = commandsFor(s, event);
      expect(cmds).toContain(NOTIFY);
    }
  });

  test("hook_installer_idempotent_across_gate_migration: second install does not duplicate", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
      gate: { gateUrl: "http://127.0.0.1:9999" },
    });
    await run(["install"], h.deps);
    const afterFirst = h.files.get("/agent/.claude/settings.json");
    const code = await run(["install"], h.deps);
    expect(code).toBe(0);
    expect(h.out.join("\n")).toContain("hooks already installed");
    expect(h.files.get("/agent/.claude/settings.json")).toBe(afterFirst!);
  });

  test("installed gate hook includes ATHING_SESSION_TOKEN and ATHING_SESSION_ID as shell vars", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
      gate: { gateUrl: "http://127.0.0.1:9999", sessionId: "sid2", sessionToken: "tok2" },
    });
    await run(["install"], h.deps);
    const s = settings(h.files);
    const cmds = commandsFor(s, "SessionStart");
    const cmd = cmds.find((c) => c.includes("Authorization"));
    expect(cmd).toBeDefined();
    expect(cmd).toContain("$ATHING_SESSION_TOKEN");
    expect(cmd).toContain("$ATHING_SESSION_ID");
  });

  test("uninstall removes gate-mode hooks by marker", async () => {
    const h = harness({
      fixture: "empty-settings.json",
      isTTY: false,
      gate: { gateUrl: "http://127.0.0.1:9999" },
    });
    await run(["install"], h.deps);
    const code = await run(["uninstall"], h.deps);
    expect(code).toBe(0);
    const s = settings(h.files);
    for (const event of EVENTS) expect(commandsFor(s, event)).toHaveLength(0);
    expect(h.out.join("\n")).toContain("hooks uninstalled");
  });
});
