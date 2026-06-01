import { test, expect, describe } from "bun:test";
import { transcriptPath } from "../src/transcript-path";

describe("transcriptPath", () => {
  test("returns .jsonl path under ~/.claude/projects", () => {
    const p = transcriptPath("sess-123", "/home/user/project");
    expect(p).toContain(".claude");
    expect(p).toContain("projects");
    expect(p).toEndWith("sess-123.jsonl");
  });

  test("encodes cwd slashes as dashes", () => {
    expect(transcriptPath("s1", "/Users/john/code/my-app")).toContain("Users-john-code-my-app");
  });

  test("no leading dash after encoding", () => {
    const p = transcriptPath("s1", "/foo");
    const encoded = p.split("/").at(-2)!;
    expect(encoded).not.toMatch(/^-/);
  });

  test("relative cwd is resolved to absolute before encoding", () => {
    const rel = transcriptPath("s1", "../project");
    expect(rel).not.toContain("..");
    expect(rel).toContain("project");
  });
});
