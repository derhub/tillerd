import { test, expect, describe } from "bun:test";
import { claudeCode, parseHook, transcriptPath, parseTranscriptEntry } from "../src/index";

describe("claudeCode adapter config", () => {
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

  test("hookInstall targets ~/.claude/settings.json", () => {
    expect(claudeCode.hookInstall.settingsPath).toBe("~/.claude/settings.json");
  });

  test("hookInstall covers all 6 event types", () => {
    const events = claudeCode.hookInstall.events;
    expect(events).toContain("SessionStart");
    expect(events).toContain("UserPromptSubmit");
    expect(events).toContain("PostToolUse");
    expect(events).toContain("PermissionRequest");
    expect(events).toContain("Stop");
    expect(events).toContain("SessionEnd");
    expect(events).toHaveLength(6);
  });

  test("cliVersionRange is set", () => {
    expect(typeof claudeCode.cliVersionRange).toBe("string");
    expect(claudeCode.cliVersionRange.length).toBeGreaterThan(0);
  });
});

describe("parseHook (task 7.3)", () => {
  test("SessionStart payload", () => {
    const raw = { hook_event_name: "SessionStart", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("SessionStart");
    expect(event.sessionId).toBe("sess-abc");
  });

  test("UserPromptSubmit payload", () => {
    const raw = { hook_event_name: "UserPromptSubmit", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("UserPromptSubmit");
  });

  test("PostToolUse payload inferred from tool_name", () => {
    const raw = { hook_event_name: "PostToolUse", session_id: "sess-abc", tool_name: "Bash" };
    const event = parseHook(raw);
    expect(event.type).toBe("PostToolUse");
  });

  test("PermissionRequest payload", () => {
    const raw = { hook_event_name: "PermissionRequest", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("PermissionRequest");
  });

  test("Stop payload", () => {
    const raw = { hook_event_name: "Stop", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("Stop");
  });

  test("SessionEnd payload", () => {
    const raw = { hook_event_name: "SessionEnd", session_id: "sess-abc" };
    const event = parseHook(raw);
    expect(event.type).toBe("SessionEnd");
  });

  test("raw payload is preserved", () => {
    const raw = { hook_event_name: "Stop", session_id: "sess-abc", extra: 42 };
    const event = parseHook(raw);
    expect(event.payload).toBe(raw);
  });
});

describe("transcriptPath (task 7.4)", () => {
  test("returns .jsonl path under ~/.claude/projects", () => {
    const p = transcriptPath("sess-123", "/home/user/project");
    expect(p).toContain(".claude");
    expect(p).toContain("projects");
    expect(p).toEndWith("sess-123.jsonl");
  });

  test("encodes cwd slashes as dashes", () => {
    const p = transcriptPath("s1", "/Users/john/code/my-app");
    expect(p).toContain("Users-john-code-my-app");
  });

  test("leading slash becomes empty (no leading dash)", () => {
    const p = transcriptPath("s1", "/foo");
    const parts = p.split("/");
    const encoded = parts[parts.length - 2];
    expect(encoded).not.toMatch(/^-/);
  });
});

describe("parseTranscriptEntry (task 7.5)", () => {
  test("tool_use block returns tool_use content", () => {
    const line = JSON.stringify({
      type: "assistant",
      session_id: "sess-1",
      message: {
        role: "assistant",
        content: [{ type: "tool_use", name: "Bash", input: { command: "ls" } }],
      },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("tool_use");
    if (result?.kind === "tool_use") {
      expect(result.toolName).toBe("Bash");
    }
  });

  test("usage message returns usage content", () => {
    const line = JSON.stringify({
      type: "assistant",
      session_id: "sess-1",
      message: {
        usage: { input_tokens: 100, output_tokens: 50 },
      },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("usage");
    if (result?.kind === "usage") {
      expect(result.inputTokens).toBe(100);
      expect(result.outputTokens).toBe(50);
    }
  });

  test("unrecognized line returns null", () => {
    const line = JSON.stringify({ type: "unknown_thing", session_id: "s1" });
    const result = parseTranscriptEntry(line, "s1");
    expect(result).toBeNull();
  });

  test("malformed JSON returns null", () => {
    const result = parseTranscriptEntry("not json", "s1");
    expect(result).toBeNull();
  });
});
