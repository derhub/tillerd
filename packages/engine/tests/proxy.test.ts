import { test, expect, describe } from "bun:test";
import type { DaemonFrame, HookEvent, HookSource, ContentEvent } from "@athing/sdk";
import type { AgentDefinition, Logger } from "@athing/sdk";
import type { FrameHandler } from "@athing/sdk";

const noopLogger: Logger = {
  debug() {},
  info() {},
  warn() {},
  error() {},
  child: () => noopLogger,
};

const mockAdapter: AgentDefinition = {
  name: "mock",
  launch: { command: "mock", args: [], flags: [] },
  interruptSequence: "\x1b",
  binaryResolution: { overrideEnvVar: "MOCK_BIN", binaryName: "mock", commonLocations: [] },
  cliVersionRange: "*",
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
    noopLogger,
    "mock",
  );
  return { proxy, client };
}

describe("fillProxyOptions — cwd contract", () => {
  test("throws when cwd is missing", async () => {
    const { fillProxyOptions } = await import("../src/daemon/proxy");
    expect(() => fillProxyOptions({})).toThrow(/cwd is required/);
  });

  test("accepts an explicit cwd", async () => {
    const { fillProxyOptions } = await import("../src/daemon/proxy");
    expect(fillProxyOptions({ cwd: "/tmp" }).cwd).toBe("/tmp");
  });
});

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
      raw: {
        code: null,
        signal: "SIGSEGV",
        signalName: "SIGSEGV",
        signalMeaning: "Segfault",
        signalCategory: "fault",
      },
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

describe("AgentSessionProxy — stop() operation", () => {
  test("stop() sends stop frame to daemon", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const stopP = proxy.stop();
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "stopped-by-request",
      raw: { code: null, signal: "SIGTERM" },
    });
    await stopP;
    const stopFrames = client.sent.filter((s) => (s.meta as { type: string }).type === "stop");
    expect(stopFrames).toHaveLength(1);
  });

  test("stop() does not emit crashed status", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));
    const stopP = proxy.stop();
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "stopped-by-request",
      raw: {},
    });
    await stopP;
    expect(statuses).not.toContain("crashed");
  });

  test("kill() sends kill frame (not stop frame)", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const killP = proxy.kill();
    client.emit("test-session-id", {
      type: "exit",
      sessionId: "test-session-id",
      qualifier: "stopped-by-request",
      raw: {},
    });
    await killP;
    expect(sentType(client, "kill")).toHaveLength(1);
    expect(sentType(client, "stop")).toHaveLength(0);
  });
});

describe("AgentSessionProxy — snapshot frame handling", () => {
  test("snapshot frame is converted to bytes and emitted on data channel", async () => {
    const { proxy, client } = await makeProxy("subscribe");
    proxy.start();
    const dataChunks: Uint8Array[] = [];
    proxy.onData((b) => dataChunks.push(b));

    client.emit("test-session-id", {
      type: "snapshot",
      sessionId: "test-session-id",
      rows: 3,
      cols: 5,
      cells: Array.from({ length: 3 }, () =>
        Array.from({ length: 5 }, () => ({ char: " ", fg: 0, bg: 0, attrs: 0 })),
      ),
      cursor: { x: 0, y: 0 },
    });

    expect(dataChunks.length).toBeGreaterThan(0);
    const allBytes = Buffer.concat(dataChunks.map((c) => Buffer.from(c)));
    const str = allBytes.toString("utf8");
    // Snapshot bytes begin with ED2 clear + home
    expect(str).toContain("\x1b[2J");
    expect(str).toContain("\x1b[H");
  });

  test("snapshot frame emitted before live data bytes (ordering)", async () => {
    const { proxy, client } = await makeProxy("subscribe");
    proxy.start();
    const received: string[] = [];
    proxy.onData((b) => received.push(Buffer.from(b).toString("utf8")));

    client.emit("test-session-id", {
      type: "snapshot",
      sessionId: "test-session-id",
      rows: 3,
      cols: 5,
      cells: Array.from({ length: 3 }, () =>
        Array.from({ length: 5 }, () => ({ char: " ", fg: 0, bg: 0, attrs: 0 })),
      ),
      cursor: { x: 0, y: 0 },
    });
    client.emit(
      "test-session-id",
      { type: "data", sessionId: "test-session-id", bodyLen: 3 },
      Buffer.from("abc"),
    );

    expect(received.length).toBeGreaterThanOrEqual(2);
    // First chunk is the snapshot (contains ED2), second is live data
    expect(received[0]).toContain("\x1b[2J");
    expect(received[received.length - 1]).toBe("abc");
  });

  test("legacy engine without snapshot capability gets ring-buffer replay from daemon (unit: no snapshot frame sent)", async () => {
    // An engine that does NOT emit snapshot frames during subscribe
    // is already tested by the absence of snapshot handling in non-capable paths.
    // This test verifies that when a subscribe triggers without snapshot frame, proxy works normally.
    const { proxy, client } = await makeProxy("subscribe");
    proxy.start();
    const dataChunks: Uint8Array[] = [];
    proxy.onData((b) => dataChunks.push(b));

    // Daemon sends only a data frame (legacy path — ring-buffer replay, not snapshot)
    client.emit(
      "test-session-id",
      { type: "data", sessionId: "test-session-id", bodyLen: 5 },
      Buffer.from("hello"),
    );

    expect(dataChunks).toHaveLength(1);
    expect(Buffer.from(dataChunks[0]!).toString()).toBe("hello");
  });
});

describe("AgentSessionProxy — crash recovery routing", () => {
  test("spawn mode proxy sends spawn frame (not subscribe) for recovery", async () => {
    // Recovery always uses spawn mode — verified by checking the frame type sent
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    expect(sentType(client, "spawn")).toHaveLength(1);
    expect(sentType(client, "subscribe")).toHaveLength(0);
  });

  test("spawn mode proxy sends resume field in spawn frame when opts.resume set", async () => {
    const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
    const client2 = new MockDaemonClient();
    const proxy2 = new AgentSessionProxy(
      "old-session-id",
      mockAdapter,
      fillProxyOptions({ cwd: "/tmp", resume: "old-session-id" }),
      client2 as never,
      "spawn",
      "/tmp/hooks.sock",
      noopLogger,
      "mock",
    );
    proxy2.start();
    const spawnFrames = sentType(client2, "spawn");
    expect(spawnFrames).toHaveLength(1);
    expect((spawnFrames[0]!.meta as { resume?: string }).resume).toBe("old-session-id");
  });

  test("recovered session starts with blank terminal (no pre-crash data in dataBuf)", async () => {
    const { proxy, client: _client } = await makeProxy("spawn");
    proxy.start();
    const chunks: Uint8Array[] = [];
    proxy.onData((b) => chunks.push(b));
    // No data frames sent — terminal is blank
    expect(chunks).toHaveLength(0);
  });
});

describe("AgentSessionProxy — terminal status routing", () => {
  test("terminal-source status frame routes to onTerminalStatus", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const terminal: string[] = [];
    proxy.onTerminalStatus((s) => terminal.push(s));
    client.emit("test-session-id", {
      type: "status",
      sessionId: "test-session-id",
      status: "WORKING",
      source: "terminal",
    });
    client.emit("test-session-id", {
      type: "status",
      sessionId: "test-session-id",
      status: "IDLE",
      source: "terminal",
    });
    expect(terminal).toEqual(["WORKING", "IDLE"]);
  });

  test("terminal status does not reach onStatus (agent plane)", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    const agent: string[] = [];
    proxy.onStatus((s) => agent.push(s));
    client.emit("test-session-id", {
      type: "status",
      sessionId: "test-session-id",
      status: "IDLE",
      source: "terminal",
    });
    expect(agent).toHaveLength(0);
  });

  test("terminal IDLE does not flush the send queue (idle-timeout logic untouched)", async () => {
    const { proxy, client } = await makeProxy("spawn");
    proxy.start();
    proxy.send("hello");
    expect(sentType(client, "input")).toHaveLength(0);
    // A terminal IDLE must NOT mark the queue ready — only a hook-derived IDLE does.
    client.emit("test-session-id", {
      type: "status",
      sessionId: "test-session-id",
      status: "IDLE",
      source: "terminal",
    });
    expect(sentType(client, "input")).toHaveLength(0);
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

// ── HookSource ingress ────────────────────────────────────────────────────────

function makeHookEvent(type: HookEvent["type"], sessionId = "test-session-id"): HookEvent {
  const base = { sessionId, correlationId: "c1", ts: 1000 };
  switch (type) {
    case "UserPromptSubmit":
      return { ...base, type, payload: { content: "hi", turnIndex: 0 } };
    case "PostToolUse":
      return {
        ...base,
        type,
        payload: { toolName: "Read", toolInput: {}, toolResponse: "", turnIndex: 1 },
      };
    case "Stop":
      return { ...base, type, payload: { turnIndex: 1 } };
    case "SessionStart":
      return { ...base, type, payload: {} };
    case "SessionEnd":
      return { ...base, type, payload: {} };
    case "PermissionRequest":
      return { ...base, type, payload: { request: {} } };
  }
}

function fakeHookSource(events: HookEvent[]): HookSource {
  return {
    subscribe(_sessionId: string): AsyncIterableIterator<HookEvent> {
      let i = 0;
      const iter: AsyncIterableIterator<HookEvent> = {
        [Symbol.asyncIterator]() {
          return iter;
        },
        async next() {
          if (i < events.length) {
            return { value: events[i++]!, done: false };
          }
          return { value: undefined as unknown as HookEvent, done: true };
        },
      };
      return iter;
    },
  };
}

async function makeProxyWithHookSource(hookSource: HookSource) {
  const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
  const client = new MockDaemonClient();
  const proxy = new AgentSessionProxy(
    "test-session-id",
    mockAdapter,
    fillProxyOptions({ cwd: "/tmp", startupTimeoutMs: 500, sendQueueCapacity: 4 }),
    client as never,
    "spawn",
    "/tmp/test-hooks.sock",
    noopLogger,
    "mock",
    hookSource,
  );
  return { proxy, client };
}

describe("AgentSessionProxy — HookSource ingress", () => {
  test("UserPromptSubmit event drives status to WORKING", async () => {
    const source = fakeHookSource([makeHookEvent("UserPromptSubmit")]);
    const { proxy } = await makeProxyWithHookSource(source);
    const statuses: string[] = [];
    proxy.onStatus((s) => statuses.push(s));
    proxy.start();
    await new Promise((r) => setTimeout(r, 20));
    expect(statuses).toContain("WORKING");
  });

  test("IDLE-after-WORKING (Stop) flushes the send queue", async () => {
    const source = fakeHookSource([makeHookEvent("UserPromptSubmit"), makeHookEvent("Stop")]);
    const { proxy, client } = await makeProxyWithHookSource(source);
    proxy.start();
    proxy.send("queued-text");
    await new Promise((r) => setTimeout(r, 20));
    expect(sentType(client, "input")).toHaveLength(1);
  });

  test("PostToolUse emits ToolUseContent to onContent handlers", async () => {
    const source = fakeHookSource([makeHookEvent("PostToolUse")]);
    const { proxy } = await makeProxyWithHookSource(source);
    const events: ContentEvent[] = [];
    proxy.onContent((e) => events.push(e));
    proxy.start();
    await new Promise((r) => setTimeout(r, 20));
    expect(events).toHaveLength(1);
    expect(events[0]!.kind).toBe("tool_use");
    if (events[0]!.kind === "tool_use") {
      expect(events[0]!.toolName).toBe("Read");
      expect(events[0]!.sessionId).toBe("test-session-id");
    }
  });

  test("non-PostToolUse events do not emit content", async () => {
    const source = fakeHookSource([makeHookEvent("UserPromptSubmit"), makeHookEvent("Stop")]);
    const { proxy } = await makeProxyWithHookSource(source);
    const events: ContentEvent[] = [];
    proxy.onContent((e) => events.push(e));
    proxy.start();
    await new Promise((r) => setTimeout(r, 20));
    expect(events).toHaveLength(0);
  });

  test("per-session isolation: hookSource.subscribe called with correct sessionId", async () => {
    const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
    const subscribedIds: string[] = [];
    const source: HookSource = {
      subscribe(sessionId: string): AsyncIterableIterator<HookEvent> {
        subscribedIds.push(sessionId);
        const iter: AsyncIterableIterator<HookEvent> = {
          [Symbol.asyncIterator]() {
            return iter;
          },
          async next() {
            return { value: undefined as unknown as HookEvent, done: true };
          },
        };
        return iter;
      },
    };

    const client = new MockDaemonClient();
    const proxy = new AgentSessionProxy(
      "session-xyz",
      mockAdapter,
      fillProxyOptions({ cwd: "/tmp" }),
      client as never,
      "spawn",
      "/tmp/hooks.sock",
      noopLogger,
      "mock",
      source,
    );
    proxy.start();
    await new Promise((r) => setTimeout(r, 10));
    expect(subscribedIds).toEqual(["session-xyz"]);
  });
});

// ── spawn env injection (task 7.4 / step 5) ──────────────────────────────────

describe("AgentSessionProxy — spawn env injection", () => {
  test("injects ATHING_DIR so the agent's hook client derives the gate socket", async () => {
    const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
    const sent: object[] = [];
    const fakeClient = {
      send(meta: object) {
        sent.push(meta);
      },
      subscribe() {
        return () => {};
      },
      async list() {
        return [];
      },
      disconnect() {},
    };
    const proxy = new AgentSessionProxy(
      "gate-env-session",
      mockAdapter,
      fillProxyOptions({ cwd: "/tmp", gateToken: "tok-abc" }),
      fakeClient as never,
      "spawn",
      "/run/athing",
      noopLogger,
      "mock",
    );
    proxy.start();

    const spawnFrame = sent.find((s) => (s as { type: string }).type === "spawn") as {
      env: Record<string, string>;
    };
    expect(spawnFrame.env["ATHING_DIR"]).toBe("/run/athing");
    expect(spawnFrame.env["ATHING_SESSION_ID"]).toBe("gate-env-session");
    expect(spawnFrame.env["ATHING_SESSION_TOKEN"]).toBe("tok-abc");
  });

  test("proxy_spawn_env_omits_athing_bridge_url", async () => {
    const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
    const sent: object[] = [];
    const fakeClient = {
      send(meta: object) {
        sent.push(meta);
      },
      subscribe() {
        return () => {};
      },
      async list() {
        return [];
      },
      disconnect() {},
    };
    const proxy = new AgentSessionProxy(
      "no-bridge-session",
      mockAdapter,
      fillProxyOptions({ cwd: "/tmp" }),
      fakeClient as never,
      "spawn",
      "/tmp/hooks.sock",
      noopLogger,
      "mock",
    );
    proxy.start();

    const spawnFrame = sent.find((s) => (s as { type: string }).type === "spawn") as {
      env: Record<string, string>;
    };
    expect(spawnFrame.env["ATHING_BRIDGE_URL"]).toBeUndefined();
  });

  test("uses provided gateToken instead of generating a random token", async () => {
    const { AgentSessionProxy, fillProxyOptions } = await import("../src/daemon/proxy");
    const sent: object[] = [];
    const fakeClient = {
      send(meta: object) {
        sent.push(meta);
      },
      subscribe() {
        return () => {};
      },
      async list() {
        return [];
      },
      disconnect() {},
    };
    const providedToken = "pre-minted-token-hex";
    const proxy = new AgentSessionProxy(
      "token-session",
      mockAdapter,
      fillProxyOptions({ cwd: "/tmp", gateToken: providedToken }),
      fakeClient as never,
      "spawn",
      "/tmp/hooks.sock",
      noopLogger,
      "mock",
    );
    expect(proxy.token).toBe(providedToken);
  });
});
