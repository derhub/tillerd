import type { ServiceHealthWire } from "@tillerd/client-bindings";

import { afterEach, expect, mock, test } from "bun:test";

const health: ServiceHealthWire[] = [{ name: "tillerd-gate", version: "1.0.0", state: "ready" }];
let called: string | undefined;

void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string) => {
    called = cmd;
    return health;
  },
  Channel: class {},
}));

const setDesktopHost = (on: boolean) => {
  if (on) (window as unknown as { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
  else delete (window as unknown as { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__;
};

afterEach(() => {
  called = undefined;
  setDesktopHost(false);
});

test("the desktop source snapshots through the service_health command", async () => {
  setDesktopHost(true);
  const { loadServiceHealthSource } = await import("./service-health-source");
  const source = await loadServiceHealthSource();
  const result = await source?.snapshot();
  expect(called).toBe("service_health");
  expect(result).toEqual(health);
});

test("the source is absent off the desktop host", async () => {
  setDesktopHost(false);
  const { loadServiceHealthSource } = await import("./service-health-source");
  expect(await loadServiceHealthSource()).toBeNull();
});
