import { test, expect, describe } from "bun:test";
import { transcriptPath } from "../src/transcript-path";

const HOME = "/home/user/.claude";

describe("transcriptPath", () => {
  test("returns .jsonl path under <agentHome>/projects", () => {
    const p = transcriptPath("sess-123", "/home/user/project", HOME);
    expect(p).toBe("/home/user/.claude/projects/home-user-project/sess-123.jsonl");
  });

  test("builds the path under the supplied agent-home", () => {
    const p = transcriptPath("s1", "/foo", "/custom/home");
    expect(p).toStartWith("/custom/home/projects/");
  });

  test("encodes cwd slashes as dashes", () => {
    expect(transcriptPath("s1", "/Users/john/code/my-app", HOME)).toContain(
      "Users-john-code-my-app",
    );
  });

  test("no leading dash after encoding", () => {
    const p = transcriptPath("s1", "/foo", HOME);
    const encoded = p.split("/").at(-2)!;
    expect(encoded).not.toMatch(/^-/);
  });
});
