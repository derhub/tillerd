import { test, expect } from "bun:test";
import { createOrchestratorClient, type OrchestratorHostTransport } from "./client";
import {
  ORCHESTRATOR_STATUS_EVENT,
  ORCHESTRATOR_STATUS_METHOD,
  type OrchestratorStatus,
} from "./status";

function fakeTransport(overrides: Partial<OrchestratorHostTransport> = {}) {
  const calls: { method: string; args?: Record<string, unknown> }[] = [];
  const listeners = new Map<string, (s: OrchestratorStatus) => void>();
  const transport: OrchestratorHostTransport = {
    invoke: async <T>(method: string, args?: Record<string, unknown>) => {
      calls.push({ method, args });
      return { state: "ready" } as T;
    },
    listen: async (event, handler) => {
      listeners.set(event, handler);
      return () => listeners.delete(event);
    },
    ...overrides,
  };
  return { transport, calls, listeners };
}

test("status() routes to the orchestrator method and returns the typed result", async () => {
  const { transport, calls } = fakeTransport();
  const client = createOrchestratorClient(transport);

  const status = await client.status();

  expect(calls).toEqual([{ method: ORCHESTRATOR_STATUS_METHOD, args: undefined }]);
  expect(status).toEqual({ state: "ready" });
});

test("subscribe() delivers status events emitted over the host transport", async () => {
  const { transport, listeners } = fakeTransport();
  const client = createOrchestratorClient(transport);

  const received: OrchestratorStatus[] = [];
  await client.subscribe((s) => received.push(s));

  // The host transport emits an event on its channel; the client forwards it
  // unchanged — it synthesizes nothing of its own.
  listeners.get(ORCHESTRATOR_STATUS_EVENT)?.({ state: "supervising" });
  listeners.get(ORCHESTRATOR_STATUS_EVENT)?.({ state: "ready" });

  expect(received).toEqual([{ state: "supervising" }, { state: "ready" }]);
});

test("subscribe() returns the transport's unsubscribe handle", async () => {
  const { transport, listeners } = fakeTransport();
  const client = createOrchestratorClient(transport);

  const unsubscribe = await client.subscribe(() => {});
  expect(listeners.has(ORCHESTRATOR_STATUS_EVENT)).toBe(true);

  unsubscribe();
  expect(listeners.has(ORCHESTRATOR_STATUS_EVENT)).toBe(false);
});
