/// <reference types="bun" />
import { beforeEach, mock } from "bun:test";

(globalThis as any).__tillerd_set_invoke_mock = (handler: any) => {
  (globalThis as any).__tillerd_active_invoke = handler;
};
(globalThis as any).__tillerd_clear_invoke_mock = () => {
  delete (globalThis as any).__tillerd_active_invoke;
};

// Register a single global mock for @tauri-apps/api/core before any bindings are imported.
// Test files will set the active mock dynamically using the helper methods.
void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    const active = (globalThis as any).__tillerd_active_invoke;
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

void mock.module("@tauri-apps/api/event", () => ({
  emit: async () => {},
  listen: async () => () => {},
}));

void mock.module("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    label: "main",
    setFocus: async () => {},
    close: async () => {},
    onCloseRequested: () => () => {},
    destroy: async () => {},
  }),
}));

import { setReady } from "./app/lib/test/real-bindings";
import { configure } from "@testing-library/react";

// Increase the default async timeout (for waitFor/findBy) to 8 seconds to prevent
// false-positive timeouts under GHA CPU starvation.
configure({ asyncUtilTimeout: 8000 });

// Readiness is module-global mutable state: a file that sets it false (a not-ready assertion or
// teardown) leaks false to the next file, stalling real query()s on the whenReady() gate (~1s
// timeouts). Default every test to ready; a suite asserting the not-ready -> ready transition opts
// out by calling setReady(false) at its start.
setReady(true);
beforeEach(() => setReady(true));
