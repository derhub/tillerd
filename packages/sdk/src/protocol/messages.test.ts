import { test, expect } from "bun:test";
import { parseDaemonFrame } from "./messages";
import type { StatusFrame } from "./messages";

// Conformance fixture shared with the daemon-rs emitter
// (packages/daemon-rs/src/server.rs `status_frame_matches_shared_contract`).
// This is the exact JSON daemon-rs puts on the wire for a terminal status frame.
const GOLDEN_STATUS_FRAME = {
  type: "status",
  sessionId: "s1",
  status: "IDLE",
  source: "terminal",
} as const;

test("daemon-rs status frame parses to the StatusFrame contract", () => {
  const frame = parseDaemonFrame(GOLDEN_STATUS_FRAME);
  expect(frame).not.toBeNull();
  expect(frame!.type).toBe("status");
  const status = frame as StatusFrame;
  expect(status.sessionId).toBe("s1");
  expect(status.status).toBe("IDLE");
  expect(status.source).toBe("terminal");
});

test("terminal source carries only IDLE or WORKING", () => {
  for (const value of ["IDLE", "WORKING"] as const) {
    const frame = parseDaemonFrame({ ...GOLDEN_STATUS_FRAME, status: value }) as StatusFrame;
    expect(frame.status).toBe(value);
    expect(frame.source).toBe("terminal");
  }
});
