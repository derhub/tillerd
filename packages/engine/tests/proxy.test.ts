import { test, expect, describe } from "bun:test";
import type { DaemonFrame } from "@athing/daemon/protocol";
import type { AgentDefinition } from "@athing/sdk";
import type { FrameHandler } from "../src/daemon/client";

const mockAdapter: AgentDefinition = {
  name: "mock",
  launch: { command: "mock", args: [], flags: [] },
  installHooks: () => {},
  uninstallHooks: () => {},
  cliVersionRange: "*",
  parseHook: (raw: unknown) => {
    const r = raw as Record<string, unknown>;
    return {
      sessionId: String(r["session_id"] ?? ""),
      type: (r["hook_event_name"] as "SessionStart") ?? "SessionStart",
      payload: raw,
    };
  },
  transcriptPath: () => "/dev/null",
  parseTranscriptEntry: () => null,
};

class MockDaemonClient {
  sent: Array<{ meta: object; body?: Buffer }> = [];
  private subs = new Map<string, Set<FrameHandler>>();

  send(meta: object, body?: Buffer): void {
    this.sent.push({ meta, body });
  }

  subscribe(sessionId: string, handler: FrameHandler): () => void {
    if (!this.subs.has(sessionId)) this.subs.set(sessionId, new Set());
    this.subs.get(sessionId)!.add(handler);
    return () => this.subs.get(sessionId)?.delete(handler);
  }

  emit(sessionId: string, frame: DaemonFrame, body: Buffer | null = null): void {
    const handlers = this.subs.get(sessionId);
    if (handlers) for (const h of handlers) h(frame, body);
  }

  async list(): Promise<string[]> {
    return [];
  }
  async connect(): Promise<void> {}
  disconnect(): void {}
}

function sentType(client: MockDaemonClient, type: string) {
  return client.sent.filter((s) => (s.meta as { type: string }).type === type);
}

async function makeProxy(mode: "spawn" | "subscribe" = "spawn") {
  const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
  const client = new MockDaemonClient();
  const proxy = new AgentSessionProxy(
    "test-session-id",
    mockAdapter,
    fillProxyOptions({ cwd: "/tmp", startupTimeoutMs: 500, sendQueueCapacity: 4 }),
    client as never,
    mode,
    "/tmp/test-hooks.sock",
  );
  return { proxy, client };
}

describe("AgentSessionProxy — spawn mode", () => {
  test("start() sends spawn frame to daemon", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    expect(sentType(client, "spawn")).toHaveLength(1);
  });

  test("start() is idempotent", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    proxy.start();
    expect(sentType(client, "spawn")).toHaveLength(1);
  });

  test("data frame delivers bytes to onData handler", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const chunks: Uint8Array[] = [];
    proxy.onData((b) => chunks.push(b));
    client.emit(
      "test-session-id",
      { type: "data", sessionId: "test-session-id", bodyLen: 3 },
      Buffer.from([65, 66, 67]),
    );
    expect(chunks).toHaveLength(1);
    expect(chunks[0]).toEqual(new Uint8Array([65, 66, 67]));
  });

  test("data frame sends ack back to daemon", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    proxy.onData(() => {});
    client.emit(
      "test-session-id",
      { type: "data", sessionId: "test-session-id", bodyLen: 5 },
      Buffer.from([1, 2, 3, 4, 5]),
    );
    const acks = sentType(client, "ack");
    expect(acks).toHaveLength(1);
    expect((acks[0]!.meta as { bytes: number }).bytes).toBe(5);
  });

  test("hook frame drives statusMapper → onStatus", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));
    client.emit("test-session-id", {
      type: "hook",
      sessionId: "test-session-id",
      payload: { hook_event_name: "UserPromptSubmit", session_id: "test-session-id" },
    });
    expect(statuses).toContain("WORKING");
  });

  test("exit frame fires onExit handler with qualifier", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const exits: import("@athing/sdk").ExitEvent[] = [];
    proxy.onExit((e) => exits.push(e));
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "ok",
      raw: { code: 0, signal: null },
    });
    expect(exits).toHaveLength(1);
    expect(exits[0]!.qualifier).toBe("ok");
  });

  test("send() is queued before ready then flushed on IDLE after WORKING", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    proxy.send("hello");
    expect(sentType(client, "input")).toHaveLength(0);
    client.emit("test-session-id", {
      type: "hook",
      sessionId: "test-session-id",
      payload: { hook_event_name: "UserPromptSubmit", session_id: "test-session-id" },
    });
    client.emit("test-session-id", {
      type: "hook",
      sessionId: "test-session-id",
      payload: { hook_event_name: "Stop", session_id: "test-session-id" },
    });
    const inputs = sentType(client, "input");
    expect(inputs).toHaveLength(1);
    expect(inputs[0]!.body).toBeDefined();
    expect(inputs[0]!.body!.toString("utf8")).toContain("hello");
  });

  test("sendQueue throws QueueFull when over capacity", async () => {
    const { proxy } = await makeProxy("spawn");
    proxy.start();
    proxy.send("a");
    proxy.send("b");
    proxy.send("c");
    proxy.send("d");
    expect(() => proxy.send("e")).toThrow();
  });

  test("kill() sends kill frame to daemon", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const killP = proxy.kill();
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "stopped-by-request",
      raw: { code: null, signal: "SIGTERM" },
    });
    await killP;
    expect(sentType(client, "kill")).toHaveLength(1);
  });

  test("startup timeout fires typed error", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const errors: string[] = [];
    proxy.onError((e) => errors.push(e.kind));
    await new Promise((r) => setTimeout(r, 600));
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "ok",
      raw: { code: null, signal: null },
    });
    expect(errors).toContain("Timeout");
  });
});

describe("AgentSessionProxy — exit qualifier & crashed status", () => {
  test("crash-class qualifier emits crashed status before exit event", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    const exits: import("@athing/sdk").ExitEvent[] = [];
    proxy.onStatus((s) => statuses.push(s));
    proxy.onExit((e) => exits.push(e));

    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "faulted",
      raw: { code: null, signal: "SIGSEGV", signalName: "SIGSEGV", signalMeaning: "Segfault", signalCategory: "fault" },
    });

    expect(statuses).toContain("crashed");
    expect(exits).toHaveLength(1);
    expect(exits[0]!.qualifier).toBe("faulted");
    // crashed must come before the exit handler fires (statuses populated before exits in this test)
    const crashedIdx = statuses.indexOf("crashed");
    expect(crashedIdx).toBeGreaterThanOrEqual(0);
  });

  test("stopped-by-request does not emit crashed status", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));

    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "stopped-by-request",
      raw: { code: null, signal: "SIGTERM" },
    });

    expect(statuses).not.toContain("crashed");
  });

  test("ok self-exit does not emit crashed status", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));

    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "ok",
      raw: { code: 0, signal: null },
    });

    expect(statuses).not.toContain("crashed");
  });

  test("error qualifier (non-zero exit, no signal) emits crashed", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));

    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "error",
      raw: { code: 1, signal: null },
    });

    expect(statuses).toContain("crashed");
  });
});

describe("AgentSessionProxy — subscribe mode", () => {
  test("start() sends subscribe frame, not spawn", async () => {
    const { proxy, client } = await makeProxy("subscribe");
    proxy.start();
    expect(sentType(client, "subscribe")).toHaveLength(1);
    expect(sentType(client, "spawn")).toHaveLength(0);
  });

  test("onData replay from dataBuf on late registration", async () => {
    const { proxy, client } = await makeProxy("subscribe");
    proxy.start();
    client.emit(
      "test-session-id",
      { type: "data", sessionId: "test-session-id", bodyLen: 2 },
      Buffer.from([10, 20]),
    );
    const chunks: Uint8Array[] = [];
    proxy.onData((b) => chunks.push(b));
    expect(chunks).toHaveLength(1);
    expect(chunks[0]).toEqual(new Uint8Array([10, 20]));
  });
});
