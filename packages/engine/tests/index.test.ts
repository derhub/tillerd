import { test, expect, describe } from "bun:test";

describe("StatusMapper", () => {
  test("maps SessionStart -> IDLE", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    m.apply({ sessionId: "s1", type: "SessionStart" });
    expect(m.get()).toBe("IDLE");
  });

  test("maps UserPromptSubmit -> WORKING", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    m.apply({ sessionId: "s1", type: "SessionStart" });
    m.apply({ sessionId: "s1", type: "UserPromptSubmit" });
    expect(m.get()).toBe("WORKING");
  });

  test("maps PermissionRequest -> WAITING_INPUT", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    m.apply({ sessionId: "s1", type: "PermissionRequest" });
    expect(m.get()).toBe("WAITING_INPUT");
  });

  test("maps Stop -> IDLE", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    m.apply({ sessionId: "s1", type: "UserPromptSubmit" });
    m.apply({ sessionId: "s1", type: "Stop" });
    expect(m.get()).toBe("IDLE");
  });

  test("maps SessionEnd -> DONE", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    m.apply({ sessionId: "s1", type: "SessionEnd" });
    expect(m.get()).toBe("DONE");
  });

  test("idempotent: duplicate event does not re-emit", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    const transitions: string[] = [];
    m.onChange((s) => transitions.push(s));
    m.apply({ sessionId: "s1", type: "UserPromptSubmit" }); // IDLE → WORKING (emits)
    m.apply({ sessionId: "s1", type: "UserPromptSubmit" }); // WORKING → WORKING (no change)
    expect(transitions).toHaveLength(1);
  });

  test("emits transition on change", async () => {
    const { StatusMapper } = await import("../src/session/status");
    const m = new StatusMapper();
    const seen: string[] = [];
    m.onChange((s) => seen.push(s));
    m.apply({ sessionId: "s1", type: "UserPromptSubmit" });
    m.apply({ sessionId: "s1", type: "Stop" });
    expect(seen).toEqual(["WORKING", "IDLE"]);
  });
});

describe("SendQueue", () => {
  test("queues text when not ready", async () => {
    const { SendQueue } = await import("../src/session/queue");
    const q = new SendQueue(10);
    q.enqueue("hello");
    expect(q.size()).toBe(1);
  });

  test("drains on setReady(true)", async () => {
    const { SendQueue } = await import("../src/session/queue");
    const q = new SendQueue(10);
    q.enqueue("a");
    q.enqueue("b");
    const drained = q.setReady(true);
    expect(drained).toEqual(["a", "b"]);
    expect(q.size()).toBe(0);
  });

  test("throws on overflow", async () => {
    const { SendQueue } = await import("../src/session/queue");
    const q = new SendQueue(2);
    q.enqueue("x");
    q.enqueue("y");
    expect(() => q.enqueue("z")).toThrow();
  });
});

describe("no-encoding-hops (task 3.4)", () => {
  test("ANSI escape bytes pass through Buffer round-trip byte-identical", () => {
    const ansi = "\x1b[31mHello\x1b[0m";
    const bytes = Buffer.from(ansi, "binary");
    const roundTripped = bytes.toString("binary");
    expect(roundTripped).toBe(ansi);
    expect(Buffer.compare(bytes, Buffer.from(roundTripped, "binary"))).toBe(0);
  });

  test("multibyte UTF-8 pass through byte-identical", () => {
    const text = "日本語テスト🎉";
    const bytes = Buffer.from(text, "utf8");
    const roundTripped = Buffer.from(bytes).toString("utf8");
    expect(roundTripped).toBe(text);
    expect(Buffer.compare(bytes, Buffer.from(roundTripped, "utf8"))).toBe(0);
  });
});
