import { test, expect, describe } from "bun:test";
import { parseTranscriptEntry } from "../src/parse-entry";

describe("parseTranscriptEntry", () => {
  test("tool_use block returns tool_use event", () => {
    const line = JSON.stringify({
      type: "assistant",
      session_id: "sess-1",
      message: {
        content: [{ type: "tool_use", name: "Bash", input: { command: "ls" } }],
      },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("tool_use");
    if (result?.kind === "tool_use") {
      expect(result.toolName).toBe("Bash");
    }
  });

  test("usage message returns usage event", () => {
    const line = JSON.stringify({
      type: "assistant",
      session_id: "sess-1",
      message: { usage: { input_tokens: 100, output_tokens: 50 } },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("usage");
    if (result?.kind === "usage") {
      expect(result.inputTokens).toBe(100);
      expect(result.outputTokens).toBe(50);
    }
  });

  test("usage entry includes costUSD when present", () => {
    const line = JSON.stringify({
      type: "assistant",
      session_id: "sess-1",
      costUSD: 0.005,
      message: { usage: { input_tokens: 10, output_tokens: 5 } },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    if (result?.kind === "usage") {
      expect(result.costUsd).toBe(0.005);
    }
  });

  test("tool_result with str_replace_editor returns edit event", () => {
    const line = JSON.stringify({
      type: "tool_result",
      session_id: "sess-1",
      tool_name: "str_replace_editor",
      tool_input: { path: "/src/foo.ts", old_string: "old", new_string: "new" },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("edit");
    if (result?.kind === "edit") {
      expect(result.filePath).toBe("/src/foo.ts");
      expect(result.oldContent).toBe("old");
      expect(result.newContent).toBe("new");
    }
  });

  test("tool_result with write_file returns edit event", () => {
    const line = JSON.stringify({
      type: "tool_result",
      session_id: "sess-1",
      tool_name: "write_file",
      tool_input: { file_path: "/src/bar.ts", new_string: "content" },
    });
    const result = parseTranscriptEntry(line, "sess-1");
    expect(result?.kind).toBe("edit");
    if (result?.kind === "edit") {
      expect(result.filePath).toBe("/src/bar.ts");
    }
  });

  test("falls back to sessionId param when entry has no session_id", () => {
    const line = JSON.stringify({
      type: "assistant",
      message: { content: [{ type: "tool_use", name: "Read", input: {} }] },
    });
    expect(parseTranscriptEntry(line, "fallback-id")?.sessionId).toBe("fallback-id");
  });

  test("unrecognized type returns null", () => {
    expect(
      parseTranscriptEntry(JSON.stringify({ type: "unknown", session_id: "s1" }), "s1"),
    ).toBeNull();
  });

  test("malformed JSON returns null", () => {
    expect(parseTranscriptEntry("not json", "s1")).toBeNull();
  });
});
