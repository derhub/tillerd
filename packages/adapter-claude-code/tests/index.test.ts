import { test, expect } from "bun:test";
import { claudeCodeAdapter, parseHook } from "../src/index";

test("claudeCodeAdapter exports correctly", () => {
  expect(claudeCodeAdapter.name).toBe("claude-code");
  expect(claudeCodeAdapter.description).toBe("Claude Code agent adapter");
});

test("parseHook returns event data", () => {
  const event = {
    type: "test",
    timestamp: Date.now(),
    data: { message: "hello" },
  };

  const result = parseHook(event);
  expect(result).toEqual(event);
});
