import { MutationObserver, QueryClient } from "@tanstack/react-query";
/// <reference lib="dom" />
import { afterAll, beforeAll, beforeEach, describe, expect, mock, test } from "bun:test";

(globalThis as { window?: unknown }).window ??= {};

// isDesktopHost() gates the module on `window.__TAURI_INTERNALS__`. `window` is a shared global
// across every test file in the bun process, so set the flag only for this file's lifetime and
// delete it afterwards -- leaking it makes isDesktopHost() true everywhere and breaks the
// "off the desktop host" expectations in sibling suites (service-health, CommandCenter, windows).
beforeAll(() => {
  (globalThis.window as Record<string, unknown>).__TAURI_INTERNALS__ = {};
});
afterAll(() => {
  delete (globalThis.window as Record<string, unknown>).__TAURI_INTERNALS__;
  // Drop the @tauri-apps/* module mocks so they do not leak into sibling test files.
  mock.restore();
});

// Capture every emit; expose the registered listener so a test can drive an inbound event.
const emitCalls: Array<{ event: string; payload: { source: string; keys: unknown[][] } }> = [];
let listenHandler: ((e: { payload: { source: string; keys: unknown[][] } }) => void) | null = null;

const actualTauriEvent = await import("@tauri-apps/api/event");
void mock.module("@tauri-apps/api/event", () => ({
  ...actualTauriEvent,
  emit: (event: string, payload: { source: string; keys: unknown[][] }) => {
    emitCalls.push({ event, payload });
    return Promise.resolve();
  },
  listen: (_event: string, cb: (e: { payload: { source: string; keys: unknown[][] } }) => void) => {
    listenHandler = cb;
    return Promise.resolve(() => {});
  },
}));

const actualTauriWindow = await import("@tauri-apps/api/window");
void mock.module("@tauri-apps/api/window", () => ({
  ...actualTauriWindow,
  getCurrentWindow: () => ({
    label: "main",
    isFocused: () => Promise.resolve(true),
  }),
}));

const { broadcastInvalidate, mountCrossWindowInvalidate } = await import("./crossWindowSync");
const { makeQueryClient } = await import("./queryClient");

const afterFlush = () => new Promise((r) => setTimeout(r, 120));

beforeEach(() => {
  emitCalls.length = 0;
  listenHandler = null;
});

describe("broadcastInvalidate", () => {
  test("coalesces a burst of mutations into a single emit", async () => {
    broadcastInvalidate([["sessions"]]);
    broadcastInvalidate([["projects"]]);
    await afterFlush();
    expect(emitCalls).toHaveLength(1);
    expect(emitCalls[0].payload.keys).toEqual([["sessions"], ["projects"]]);
  });

  test("dedupes repeated keys within the flush window", async () => {
    broadcastInvalidate([["sessions"], ["sessions"]]);
    broadcastInvalidate([["sessions"]]);
    await afterFlush();
    expect(emitCalls[0].payload.keys).toEqual([["sessions"]]);
  });

  test("tags each emit with the source window label", async () => {
    broadcastInvalidate([["sessions"]]);
    await afterFlush();
    expect(emitCalls[0].payload.source).toBe("main");
  });
});

describe("mountCrossWindowInvalidate", () => {
  test("invalidates matching queries when a sibling window broadcasts", async () => {
    const client = new QueryClient();
    const invalidateQueries = mock(() => Promise.resolve());
    client.invalidateQueries = invalidateQueries;
    await mountCrossWindowInvalidate(client);

    listenHandler?.({ payload: { source: "other", keys: [["sessions"]] } });
    await afterFlush();

    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["sessions"] });
  });

  test("ignores events broadcast by this same window", async () => {
    const client = new QueryClient();
    const invalidateQueries = mock(() => Promise.resolve());
    client.invalidateQueries = invalidateQueries;
    await mountCrossWindowInvalidate(client);

    listenHandler?.({ payload: { source: "main", keys: [["sessions"]] } });
    await afterFlush();

    expect(invalidateQueries).not.toHaveBeenCalled();
  });
});

describe("invalidation counts: same window vs child window", () => {
  function countingClient() {
    const client = makeQueryClient();
    const real = client.invalidateQueries.bind(client);
    let count = 0;
    client.invalidateQueries = (...args: Parameters<typeof real>) => {
      count += 1;
      return real(...args);
    };
    return { client, count: () => count };
  }

  test("same window invalidates once locally and not again from its own broadcast", async () => {
    const { client, count } = countingClient();
    await mountCrossWindowInvalidate(client);

    const observer = new MutationObserver(client, {
      mutationFn: async () => "ok",
      meta: { invalidates: [["projects"]] },
    });
    await observer.mutate();

    expect(count()).toBe(1);

    await afterFlush();
    expect(emitCalls).toHaveLength(1);
    listenHandler?.({ payload: emitCalls[0].payload });
    await afterFlush();
    expect(count()).toBe(1);
  });

  test("child window invalidates once per unique key for a coalesced mutation burst", async () => {
    const { client, count } = countingClient();
    await mountCrossWindowInvalidate(client);

    listenHandler?.({
      payload: { source: "other", keys: [["sessions"], ["sessions"], ["projects"]] },
    });
    await afterFlush();

    expect(count()).toBe(2);
  });

  test("child window collapses two inbound events in one flush into a single pass", async () => {
    const { client, count } = countingClient();
    await mountCrossWindowInvalidate(client);

    listenHandler?.({ payload: { source: "other", keys: [["sessions"]] } });
    listenHandler?.({ payload: { source: "other", keys: [["sessions"]] } });
    await afterFlush();

    expect(count()).toBe(1);
  });
});
