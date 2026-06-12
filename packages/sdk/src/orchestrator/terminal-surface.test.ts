import { expect, test } from "bun:test";
import {
  SURFACE_CREATE,
  SURFACE_DETACH,
  SURFACE_EXIT_EVENT,
  SURFACE_INPUT,
  SURFACE_RESIZE,
  SURFACE_STATUS_EVENT,
  type SurfaceExitEvent,
  type SurfaceStatusEvent,
  type TerminalSurfaceTransport,
  createTerminalSurfaceClient,
} from "./terminal-surface";

// ---------------------------------------------------------------------------
// Fake in-memory transport
// ---------------------------------------------------------------------------

interface InvokeCall {
  command: string;
  args?: Record<string, unknown>;
}

interface FakeChannel {
  push(bytes: Uint8Array): void;
}

function makeFakeTransport(surfaceId = "surf-123"): {
  transport: TerminalSurfaceTransport;
  invokes: InvokeCall[];
  listeners: Map<string, Array<(payload: unknown) => void>>;
  lastChannel: FakeChannel | null;
} {
  const invokes: InvokeCall[] = [];
  const listeners = new Map<string, Array<(payload: unknown) => void>>();
  let lastChannel: FakeChannel | null = null;

  const transport: TerminalSurfaceTransport = {
    invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      invokes.push({ command, args });
      // surface_create returns the canned surfaceId
      if (command === SURFACE_CREATE) return Promise.resolve(surfaceId as T);
      return Promise.resolve(undefined as T);
    },
    listen<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
      const handlers = listeners.get(event) ?? [];
      handlers.push(handler as (payload: unknown) => void);
      listeners.set(event, handlers);
      return Promise.resolve(() => {
        const hs = listeners.get(event) ?? [];
        const idx = hs.indexOf(handler as (payload: unknown) => void);
        if (idx !== -1) hs.splice(idx, 1);
      });
    },
    createByteChannel(onBytes: (bytes: Uint8Array) => void): unknown {
      const channel: FakeChannel = { push: onBytes };
      lastChannel = channel;
      return channel;
    },
  };

  return {
    transport,
    invokes,
    listeners,
    get lastChannel() {
      return lastChannel;
    },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test("create() creates a byte channel, invokes surface_create with sessionId/cols/rows/cwd, and returns the surfaceId", async () => {
  const { transport, invokes } = makeFakeTransport("surf-abc");
  const client = createTerminalSurfaceClient(transport);

  const id = await client.create(
    { sessionId: "sess-1", cols: 80, rows: 24, cwd: "/home/user" },
    () => {},
  );

  expect(id).toBe("surf-abc");
  expect(invokes).toHaveLength(1);
  const call0 = invokes[0]!;
  expect(call0.command).toBe(SURFACE_CREATE);
  expect(call0.args?.sessionId).toBe("sess-1");
  expect(call0.args?.cols).toBe(80);
  expect(call0.args?.rows).toBe(24);
  expect(call0.args?.cwd).toBe("/home/user");
  expect(call0.args?.channel).toBeDefined();
});

test("create() routes channel bytes to onBytes", async () => {
  const fake = makeFakeTransport();
  const client = createTerminalSurfaceClient(fake.transport);

  const received: Uint8Array[] = [];
  await client.create({ sessionId: "sess-1", cols: 80, rows: 24 }, (b) => received.push(b));

  const chunk = new Uint8Array([72, 101, 108, 108, 111]);
  fake.lastChannel!.push(chunk);

  expect(received).toHaveLength(1);
  expect(received[0]).toEqual(chunk);
});

test("input() invokes surface_input with surfaceId and number[] bytes", async () => {
  const { transport, invokes } = makeFakeTransport();
  const client = createTerminalSurfaceClient(transport);

  const bytes = new Uint8Array([65, 66, 67]);
  await client.input("surf-123", bytes);

  expect(invokes).toHaveLength(1);
  const call0 = invokes[0]!;
  expect(call0.command).toBe(SURFACE_INPUT);
  expect(call0.args?.surfaceId).toBe("surf-123");
  expect(call0.args?.bytes).toEqual([65, 66, 67]);
});

test("resize() invokes surface_resize with surfaceId, cols, and rows", async () => {
  const { transport, invokes } = makeFakeTransport();
  const client = createTerminalSurfaceClient(transport);

  await client.resize("surf-123", 120, 40);

  expect(invokes).toHaveLength(1);
  const call0 = invokes[0]!;
  expect(call0.command).toBe(SURFACE_RESIZE);
  expect(call0.args).toEqual({ surfaceId: "surf-123", cols: 120, rows: 40 });
});

test("detach() invokes surface_detach with surfaceId", async () => {
  const { transport, invokes } = makeFakeTransport();
  const client = createTerminalSurfaceClient(transport);

  await client.detach("surf-123");

  expect(invokes).toHaveLength(1);
  const call0 = invokes[0]!;
  expect(call0.command).toBe(SURFACE_DETACH);
  expect(call0.args).toEqual({ surfaceId: "surf-123" });
});

test("onStatus() subscribes to surface://status and forwards payloads", async () => {
  const { transport, listeners } = makeFakeTransport();
  const client = createTerminalSurfaceClient(transport);

  const received: SurfaceStatusEvent[] = [];
  await client.onStatus((e) => received.push(e));

  const event: SurfaceStatusEvent = { surfaceId: "surf-123", status: "running" };
  listeners.get(SURFACE_STATUS_EVENT)?.forEach((h) => h(event));

  expect(received).toHaveLength(1);
  expect(received[0]).toEqual(event);
});

test("onExit() subscribes to surface://exit and forwards payloads", async () => {
  const { transport, listeners } = makeFakeTransport();
  const client = createTerminalSurfaceClient(transport);

  const received: SurfaceExitEvent[] = [];
  await client.onExit((e) => received.push(e));

  const event: SurfaceExitEvent = { surfaceId: "surf-123", qualifier: "success" };
  listeners.get(SURFACE_EXIT_EVENT)?.forEach((h) => h(event));

  expect(received).toHaveLength(1);
  expect(received[0]).toEqual(event);
});
