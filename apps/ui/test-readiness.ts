/// <reference types="bun" />
import { beforeEach, mock } from "bun:test";
import { AsyncLocalStorage } from "node:async_hooks";

const invokeMockStorage = new AsyncLocalStorage<any>();
(globalThis as any).__tillerd_set_invoke_mock = (handler: any) => {
  invokeMockStorage.enterWith(handler);
};
(globalThis as any).__tillerd_clear_invoke_mock = () => {
  invokeMockStorage.enterWith(undefined);
};

// Register a single global mock for @tauri-apps/api/core before any bindings are imported.
// Test files will set the active mock dynamically using the AsyncLocalStorage helper.
void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    const active = invokeMockStorage.getStore();
    if (active) {
      const res = await active(cmd, args);
      if (res !== undefined) return res;
    }
    return null;
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

import { setReady } from "./app/lib/test/real-bindings";

// Readiness is module-global mutable state: a file that sets it false (a not-ready assertion or
// teardown) leaks false to the next file, stalling real query()s on the whenReady() gate (~1s
// timeouts). Default every test to ready; a suite asserting the not-ready -> ready transition opts
// out by calling setReady(false) at its start.
setReady(true);
beforeEach(() => setReady(true));
