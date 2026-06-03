import { test, expect, describe } from "bun:test";
import { claudeCode, setup } from "../src/index";

describe("claudeCode adapter", () => {
  test("name is claude-code", () => {
    expect(claudeCode.name).toBe("claude-code");
  });

  test("launch command is claude", () => {
    expect(claudeCode.launch.command).toBe("claude");
  });

  test("launch args include --session-id placeholder", () => {
    expect(claudeCode.launch.args).toContain("{id}");
  });

  test("launch flags include --dangerously-skip-permissions", () => {
    expect(claudeCode.launch.flags).toContain("--dangerously-skip-permissions");
  });

  test("cliVersionRange is set", () => {
    expect(claudeCode.cliVersionRange.length).toBeGreaterThan(0);
  });

  test("interruptSequence is ESC", () => {
    expect(claudeCode.interruptSequence).toBe("\x1b");
  });

  test("exposes declarative binary-resolution policy, not an I/O method", () => {
    expect(claudeCode.binaryResolution.binaryName).toBe("claude");
    expect(claudeCode.binaryResolution.overrideEnvVar).toBe("CLAUDE_CODE_EXECUTABLE");
    expect(claudeCode.binaryResolution.commonLocations.length).toBeGreaterThan(0);
    expect((claudeCode as unknown as Record<string, unknown>)["resolveCommand"]).toBeUndefined();
  });

  test("engine-facing definition carries no setup or hook-plan members", () => {
    const record = claudeCode as unknown as Record<string, unknown>;
    expect(record["hookInstall"]).toBeUndefined();
    expect(record["planHookInstall"]).toBeUndefined();
    expect(record["planHookUninstall"]).toBeUndefined();
    expect(record["installHooks"]).toBeUndefined();
    expect(record["setup"]).toBeUndefined();
  });

  test("setup is a separate sibling export with install/uninstall", () => {
    expect(typeof setup.install).toBe("function");
    expect(typeof setup.uninstall).toBe("function");
  });
});
