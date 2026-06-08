import { test, expect, describe, afterEach } from "bun:test";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { unlinkSync } from "node:fs";
import { HOOK_SUBSCRIPTION_WIRE_VERSION, encodeSubscriptionFrame } from "@athing/sdk";
import { subscribeToSession, GateNegotiationError } from "../src/gate-client";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ServerSocket = any;

// ── helpers ──────────────────────────────────────────────────────────────────

function frameJson(obj: unknown): Uint8Array {
  return encodeSubscriptionFrame(new TextEncoder().encode(JSON.stringify(obj)));
}

function readyFrame(): Uint8Array {
  return frameJson({ frame: "ready", wireVersion: HOOK_SUBSCRIPTION_WIRE_VERSION });
}

function errorFrame(reason: string): Uint8Array {
  return frameJson({ frame: "error", reason });
}

function eventFrame(event: unknown): Uint8Array {
  return frameJson({ frame: "event", event });
}

function makeEvent(type: string, extra?: Record<string, unknown>) {
  return {
    sessionId: "sess-a",
    correlationId: "corr-1",
    ts: 1000,
    type,
    payload: extra ?? {},
  };
}

// ── fake server factory ───────────────────────────────────────────────────────

interface FakeServer {
  socketPath: string;
  stop(): void;
}

function startFakeGate(
  handler: (socket: ServerSocket, send: (data: Uint8Array) => void) => void,
): FakeServer {
  const socketPath = join(
    tmpdir(),
    `gate-test-${Date.now()}-${Math.random().toString(36).slice(2)}.sock`,
  );

  const server = Bun.listen({
    unix: socketPath,
    socket: {
      open(socket) {
        handler(socket, (data) => socket.write(data));
      },
      data() {},
      close() {},
      error() {},
    },
  });

  return {
    socketPath,
    stop() {
      server.stop(true);
      try {
        unlinkSync(socketPath);
      } catch {
        /* already gone */
      }
    },
  };
}

// ── tests ─────────────────────────────────────────────────────────────────────

describe("gate-client — negotiate and event delivery", () => {
  let gate: FakeServer;
  afterEach(() => gate?.stop());

  test("resolves and yields events after ready handshake", async () => {
    gate = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(eventFrame(makeEvent("UserPromptSubmit", { content: "hello", turnIndex: 0 })));
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.done).toBe(false);
    expect(result.value.type).toBe("UserPromptSubmit");
    expect(result.value.sessionId).toBe("sess-a");
  });

  test("preserves correlationId on delivered event", async () => {
    gate = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(eventFrame({ ...makeEvent("Stop", { turnIndex: 1 }), correlationId: "corr-xyz" }));
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.value.correlationId).toBe("corr-xyz");
  });

  test("delivers multiple events in order", async () => {
    gate = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(
        eventFrame({
          ...makeEvent("UserPromptSubmit", { content: "a", turnIndex: 0 }),
          correlationId: "c1",
        }),
      );
      send(
        eventFrame({
          ...makeEvent("PostToolUse", {
            toolName: "Read",
            toolInput: {},
            toolResponse: "",
            turnIndex: 1,
          }),
          correlationId: "c2",
        }),
      );
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const r1 = await iter.next();
    const r2 = await iter.next();
    expect(r1.value.correlationId).toBe("c1");
    expect(r2.value.correlationId).toBe("c2");
  });

  test("done when server closes connection", async () => {
    gate = startFakeGate((socket, send) => {
      send(readyFrame());
      // close shortly after
      setTimeout(() => socket.end(), 10);
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.done).toBe(true);
  });

  test("error frame closes the iterator", async () => {
    gate = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(errorFrame("session gone"));
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.done).toBe(true);
  });
});

describe("gate-client — version mismatch rejection", () => {
  let gate: FakeServer;
  afterEach(() => gate?.stop());

  test("rejects with GateNegotiationError when wire version mismatches", async () => {
    gate = startFakeGate((_socket, send) => {
      send(frameJson({ frame: "ready", wireVersion: 99 }));
    });

    await expect(
      subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" }),
    ).rejects.toBeInstanceOf(GateNegotiationError);
  });

  test("rejects with GateNegotiationError when gate sends error frame as first frame", async () => {
    gate = startFakeGate((_socket, send) => {
      send(errorFrame("unsupported wire version"));
    });

    await expect(
      subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" }),
    ).rejects.toBeInstanceOf(GateNegotiationError);
  });
});

describe("gate-client — partial frame reassembly", () => {
  let gate: FakeServer;
  afterEach(() => gate?.stop());

  test("assembles event split across two TCP chunks", async () => {
    gate = startFakeGate((socket, send) => {
      send(readyFrame());
      const full = eventFrame(makeEvent("Stop", { turnIndex: 3 }));
      const mid = Math.floor(full.length / 2);
      socket.write(full.slice(0, mid));
      setTimeout(() => socket.write(full.slice(mid)), 5);
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.done).toBe(false);
    expect(result.value.type).toBe("Stop");
  });

  test("assembles ready frame split across three chunks", async () => {
    gate = startFakeGate((socket) => {
      const full = readyFrame();
      const third = Math.floor(full.length / 3);
      socket.write(full.slice(0, third));
      setTimeout(() => socket.write(full.slice(third, 2 * third)), 2);
      setTimeout(() => socket.write(full.slice(2 * third)), 4);
      setTimeout(() => socket.write(eventFrame(makeEvent("SessionEnd", { reason: "done" }))), 6);
    });

    const iter = await subscribeToSession({ socketPath: gate.socketPath, sessionId: "sess-a" });
    const result = await iter.next();
    expect(result.value.type).toBe("SessionEnd");
  });
});

describe("gate-client — per-session isolation", () => {
  let gate1: FakeServer;
  let gate2: FakeServer;
  afterEach(() => {
    gate1?.stop();
    gate2?.stop();
  });

  test("two subscribers on separate sockets get independent events", async () => {
    gate1 = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(
        eventFrame({
          ...makeEvent("UserPromptSubmit", { content: "from-1", turnIndex: 0 }),
          sessionId: "sess-1",
        }),
      );
    });

    gate2 = startFakeGate((_socket, send) => {
      send(readyFrame());
      send(eventFrame({ ...makeEvent("Stop", {}), sessionId: "sess-2" }));
    });

    const [iter1, iter2] = await Promise.all([
      subscribeToSession({ socketPath: gate1.socketPath, sessionId: "sess-1" }),
      subscribeToSession({ socketPath: gate2.socketPath, sessionId: "sess-2" }),
    ]);

    const [r1, r2] = await Promise.all([iter1.next(), iter2.next()]);
    expect(r1.value.sessionId).toBe("sess-1");
    expect(r1.value.type).toBe("UserPromptSubmit");
    expect(r2.value.sessionId).toBe("sess-2");
    expect(r2.value.type).toBe("Stop");
  });
});
