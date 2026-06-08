import { test, expect, describe } from "bun:test";
import { hookEventToContent } from "../src/hook-content";
import type { HookEvent } from "../src/types/events";

function base(_type: HookEvent["type"]): { sessionId: string; correlationId: string; ts: number } {
  return { sessionId: "sess-1", correlationId: "c1", ts: 1000 };
}

describe("hookEventToContent", () => {
  test("PostToolUse maps to ToolUseContent", () => {
    const event: HookEvent = {
      ...base("PostToolUse"),
      type: "PostToolUse",
      payload: { toolName: "Read", toolInput: { file: "x" }, toolResponse: "ok", turnIndex: 0 },
    };
    const result = hookEventToContent(event);
    expect(result).not.toBeNull();
    expect(result!.kind).toBe("tool_use");
    if (result!.kind === "tool_use") {
      expect(result!.sessionId).toBe("sess-1");
      expect(result!.toolName).toBe("Read");
      expect(result!.toolInput).toEqual({ file: "x" });
    }
  });

  test("UserPromptSubmit returns null", () => {
    const event: HookEvent = {
      ...base("UserPromptSubmit"),
      type: "UserPromptSubmit",
      payload: { content: "hello", turnIndex: 0 },
    };
    expect(hookEventToContent(event)).toBeNull();
  });

  test("Stop returns null", () => {
    const event: HookEvent = {
      ...base("Stop"),
      type: "Stop",
      payload: { turnIndex: 1 },
    };
    expect(hookEventToContent(event)).toBeNull();
  });

  test("SessionStart returns null", () => {
    const event: HookEvent = {
      ...base("SessionStart"),
      type: "SessionStart",
      payload: {},
    };
    expect(hookEventToContent(event)).toBeNull();
  });

  test("SessionEnd returns null", () => {
    const event: HookEvent = {
      ...base("SessionEnd"),
      type: "SessionEnd",
      payload: {},
    };
    expect(hookEventToContent(event)).toBeNull();
  });

  test("PermissionRequest returns null", () => {
    const event: HookEvent = {
      ...base("PermissionRequest"),
      type: "PermissionRequest",
      payload: { request: {} },
    };
    expect(hookEventToContent(event)).toBeNull();
  });
});
