import { test, expect, describe } from "bun:test";
import { parseHook } from "../src/parse-hook";

describe("parseHook", () => {
  test("SessionStart payload", () => {
    const raw = { hook_event_name: "SessionStart", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("SessionStart");
    expect(event.sessionId).toBe("sess-abc");
  });

  test("UserPromptSubmit payload", () => {
    const raw = { hook_event_name: "UserPromptSubmit", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("UserPromptSubmit");
  });

  test("PostToolUse payload from hook_event_name", () => {
    const raw = { hook_event_name: "PostToolUse", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("PostToolUse");
  });

  test("PermissionRequest payload", () => {
    const raw = { hook_event_name: "PermissionRequest", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("PermissionRequest");
  });

  test("Stop payload", () => {
    const raw = { hook_event_name: "Stop", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("Stop");
  });

  test("SessionEnd payload", () => {
    const raw = { hook_event_name: "SessionEnd", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("SessionEnd");
  });

  test("raw payload is preserved", () => {
    const raw = { hook_event_name: "Stop", session_id: "sess-abc", extra: 42 };
    expect(parseHook(raw).payload).toBe(raw);
  });

  test("infers PostToolUse from tool_name when hook_event_name absent", () => {
    const raw = { tool_name: "Bash", session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("PostToolUse");
  });

  test("infers PermissionRequest from permission_request key when hook_event_name absent", () => {
    const raw = { permission_request: {}, session_id: "sess-abc" };
    expect(parseHook(raw).type).toBe("PermissionRequest");
  });

  test("falls back to SessionStart when nothing matches", () => {
    expect(parseHook({ session_id: "sess-abc" }).type).toBe("SessionStart");
  });

  test("handles null raw gracefully", () => {
    const event = parseHook(null);
    expect(event.sessionId).toBe("");
    expect(event.type).toBe("SessionStart");
  });
});
