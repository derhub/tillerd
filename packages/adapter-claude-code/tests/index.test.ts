import { test, expect, describe } from "bun:test";
import { claudeCode } from "../src/index";

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

  test("installHooks is a function", () => {
    expect(typeof claudeCode.installHooks).toBe("function");
  });

  test("uninstallHooks is a function", () => {
    expect(typeof claudeCode.uninstallHooks).toBe("function");
  });
});
