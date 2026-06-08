import { test, expect } from "bun:test";
import { AtError } from "../src/index";
import type { SessionStatus, HookEvent, HookEventType, ContentEvent } from "../src/index";

test("SessionStatus values are valid string literals", () => {
  const statuses: SessionStatus[] = ["IDLE", "WORKING", "WAITING_INPUT", "DONE"];
  expect(statuses).toHaveLength(4);
});

test("HookEventType values are valid string literals", () => {
  const types: HookEventType[] = [
    "SessionStart",
    "UserPromptSubmit",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
    "SessionEnd",
  ];
  expect(types).toHaveLength(6);
});

test("HookEvent shape is correct", () => {
  const event: HookEvent = {
    sessionId: "sess-123",
    correlationId: "corr-1",
    ts: 1_700_000_000_000,
    type: "SessionStart",
    payload: { cwd: "/repo" },
  };
  expect(event.sessionId).toBe("sess-123");
  expect(event.type).toBe("SessionStart");
});

test("ContentEvent discriminant union works", () => {
  const toolUse: ContentEvent = {
    kind: "tool_use",
    sessionId: "s1",
    toolName: "Bash",
    toolInput: {},
  };
  const edit: ContentEvent = { kind: "edit", sessionId: "s1", filePath: "/tmp/a.ts" };
  const usage: ContentEvent = { kind: "usage", sessionId: "s1", inputTokens: 10, outputTokens: 5 };
  expect(toolUse.kind).toBe("tool_use");
  expect(edit.kind).toBe("edit");
  expect(usage.kind).toBe("usage");
});

test("AtError carries kind and name", () => {
  const err = new AtError("BinaryNotFound", "claude not on PATH");
  expect(err.kind).toBe("BinaryNotFound");
  expect(err.name).toBe("BinaryNotFound");
  expect(err.message).toBe("claude not on PATH");
  expect(err instanceof Error).toBe(true);
});

test("AtError defaults message to kind", () => {
  const err = new AtError("Timeout");
  expect(err.message).toBe("Timeout");
});
