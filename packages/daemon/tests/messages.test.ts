import { test, expect, describe } from "bun:test";
import { parseClientFrame, parseDaemonFrame } from "../src/protocol/messages";

describe("parseClientFrame", () => {
  test("valid list frame returns parsed object", () => {
    expect(parseClientFrame({ type: "list" })).toEqual({ type: "list" });
  });

  test("valid spawn frame returns parsed object", () => {
    const frame = {
      type: "spawn" as const,
      sessionId: "s1",
      command: "claude",
      args: [] as string[],
      flags: [] as string[],
      hookSocketPath: "/tmp/hooks.sock",
      token: "tok",
      cols: 80,
      rows: 24,
      cwd: "/home/user",
    };
    expect(parseClientFrame(frame)).toEqual(frame);
  });

  test("valid ack frame returns parsed object", () => {
    expect(parseClientFrame({ type: "ack", sessionId: "s1", bytes: 1024 })).toEqual({
      type: "ack",
      sessionId: "s1",
      bytes: 1024,
    });
  });

  test("valid resize frame returns parsed object", () => {
    expect(parseClientFrame({ type: "resize", sessionId: "s1", cols: 80, rows: 24 })).toEqual({
      type: "resize",
      sessionId: "s1",
      cols: 80,
      rows: 24,
    });
  });

  test("unknown type returns null", () => {
    expect(parseClientFrame({ type: "unknown" })).toBeNull();
  });

  test("spawn with missing required field returns null", () => {
    expect(parseClientFrame({ type: "spawn", sessionId: "s1" })).toBeNull();
  });

  test("null input returns null", () => {
    expect(parseClientFrame(null)).toBeNull();
  });

  test("string input returns null", () => {
    expect(parseClientFrame("hello")).toBeNull();
  });

  test("number input returns null", () => {
    expect(parseClientFrame(42)).toBeNull();
  });
});

describe("parseDaemonFrame", () => {
  test("valid hello-ack frame returns parsed object", () => {
    expect(parseDaemonFrame({ type: "hello-ack", version: 1, daemonVersion: "0.1.0" })).toEqual({
      type: "hello-ack",
      version: 1,
      daemonVersion: "0.1.0",
    });
  });

  test("valid error frame with optional sessionId returns parsed object", () => {
    const frame = { type: "error" as const, code: "ENOTFOUND", message: "not found", sessionId: "s1" };
    expect(parseDaemonFrame(frame)).toEqual(frame);
  });

  test("valid error frame without sessionId returns parsed object", () => {
    expect(parseDaemonFrame({ type: "error", code: "EPROTO", message: "bad proto" })).toEqual({
      type: "error",
      code: "EPROTO",
      message: "bad proto",
    });
  });

  test("unknown type returns null", () => {
    expect(parseDaemonFrame({ type: "unknown" })).toBeNull();
  });

  test("null input returns null", () => {
    expect(parseDaemonFrame(null)).toBeNull();
  });
});
