import type { ServiceHealthWire } from "@tillerd/client-bindings";

import { test, expect } from "bun:test";

import { aggregateHealthState } from "./aggregate";

function svc(name: string, state: ServiceHealthWire["state"]): ServiceHealthWire {
  return { name, version: state === "unavailable" ? null : "1.0.0", state };
}

test("aggregate is ready when orchestrator is ready and every service is ready", () => {
  const state = aggregateHealthState("ready", [svc("gate", "ready"), svc("daemon", "ready")]);
  expect(state).toBe("ready");
});

test("aggregate is failed when any service is unavailable", () => {
  const state = aggregateHealthState("ready", [svc("gate", "ready"), svc("daemon", "unavailable")]);
  expect(state).toBe("failed");
});

test("aggregate is failed when the orchestrator itself failed", () => {
  expect(aggregateHealthState("error", [svc("gate", "ready")])).toBe("failed");
});

test("aggregate is failed when a service is on the wrong version", () => {
  expect(aggregateHealthState("ready", [svc("gate", "versionMismatch")])).toBe("failed");
});

test("aggregate is starting while a service is still starting and none failed", () => {
  expect(aggregateHealthState("ready", [svc("gate", "ready"), svc("daemon", "starting")])).toBe(
    "starting",
  );
});

test("aggregate is starting while the orchestrator is still booting", () => {
  expect(aggregateHealthState("booting", [])).toBe("starting");
});

test("a draining service reads as starting, not failed", () => {
  expect(aggregateHealthState("ready", [svc("gate", "draining")])).toBe("starting");
});
