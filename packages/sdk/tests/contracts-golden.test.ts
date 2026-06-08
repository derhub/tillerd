import { test, expect } from "bun:test";
import type { HookEvent } from "../src/index";

// The same canonical events the Rust `contracts-rs` round-trips against. If the two
// encodings ever diverge, one side stops matching this shared fixture.
const events: HookEvent[] = [
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000000,
    type: "SessionStart",
    payload: { cwd: "/repo", client: "cli", cliVersion: "1.2.3" },
  },
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000001,
    type: "UserPromptSubmit",
    payload: { content: "hello", turnIndex: 0 },
  },
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000002,
    type: "PostToolUse",
    payload: {
      toolName: "Bash",
      toolInput: { command: "ls" },
      toolResponse: "file.txt",
      turnIndex: 1,
    },
  },
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000003,
    type: "PermissionRequest",
    payload: { toolName: "Bash", request: { command: "rm -rf /" } },
  },
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000004,
    type: "Stop",
    payload: { turnIndex: 1 },
  },
  {
    sessionId: "s1",
    correlationId: "c1",
    ts: 1700000000005,
    type: "SessionEnd",
    payload: { reason: "clean" },
  },
];

test("the sdk HookEvent encoding matches the contracts-rs golden wire", async () => {
  const fixtures = await Bun.file(
    new URL("../../contracts-rs/fixtures/hook_events.json", import.meta.url),
  ).json();
  expect(events).toEqual(fixtures);
});
